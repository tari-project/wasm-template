import {useEffect, useState} from "react";
// import { WalletConnectTariSigner } from "@tari-project/wallet-connect-signer";
import { defaultPermissions, TariConnectButton } from "@tari-project/react-mui-connect-button";
import {
  AccountData,
  TransactionBuilder,
  buildTransactionRequest,
  submitAndWaitForTransaction,
  TariSigner,
  getCborValueByPath,
  TariPermissions, SubstateMetadata,
} from "@tari-project/tarijs-all";
import reactLogo from './assets/react.svg';
import viteLogo from '/vite.svg';
import './App.css';
import {ComponentHeader, NetworkByte, substateIdToString} from "@tari-project/typescript-bindings";


// Template address for creating a new component
const TEMPLATE_ADDRESS = "4e58528c0ab45e0201c617d6860752e23ca02c331235e8907a61c420b7e6465f";

// Create the fee amount (e.g., 2000 micro XTR)
const fee = 2000;

function App() {
  const [errorMessage, setErrorMessage] = useState<string | null>(null); // Track any connection errors
  const [isSubmitting, setIsSubmitting] = useState(false); // Track submission state
  const [txResult, setTxResult] = useState<any>(null); // Store transaction result
  const [showFullJson, setShowFullJson] = useState(false); // Toggle for showing full JSON
  const [substates, setSubstates] = useState<SubstateMetadata[]>([]); // Store the list of substates
  const [showSubstates, setShowSubstates] = useState(false); // Toggle for showing substates
  const [counterComponentAddress, setCounterComponentAddress] = useState<string>(""); // Store the entered substate address
  const [value, setValue] = useState<bigint | null>(null); // Store the entered substate address

  const WC_PROJECT_ID =  "78f3485d08b9640a087cbcea000e1f8b";
  
  const [signer, setSigner] = useState<TariSigner | null>(null);
  const [account, setAccount] = useState<AccountData | null>(null);

  const onConnected = async (signer: TariSigner) => {
    setSigner(signer);
    const account = await signer.getAccount();
    setAccount(account);
  };

  const wcParams = {
    projectId: WC_PROJECT_ID,
    requiredPermissions: defaultPermissions().getPermissions(),
  };

  const createCounter = async () => {
    if (!signer) {
      setErrorMessage("Signer is not available. Please connect to the wallet first.");
      return;
    }

    setIsSubmitting(true);  // Start the transaction submission process
    setErrorMessage(null);

    try {

      // Get the account executing the transaction
      const account = await signer.getAccount();

      // Initialize the TransactionBuilder
      let builder = new TransactionBuilder(NetworkByte.Igor);

      const transaction = builder
          // Allocate a new component address for the counter
          .allocateAddress("Component", "counter_component") // Allocate a new component address
          // Specify that the fee will be paid from the account
          .feeTransactionPayFromComponent(account.address, fee.toString())
          // Call the template function to create a new component
          .callFunction(
            {
              templateAddress: TEMPLATE_ADDRESS,
              // Create a new counter - you can also use "new" but then you would not be able to increase the counter in the same transaction
              functionName: "with_address",
            },
            [{Workspace: "counter_component"}]  // Parameters to pass to the function
          )
          // Increase it once just for fun
          .callMethod({
            fromWorkspace: "counter_component",  // Use the allocated component address
            methodName: "increase"
          }, [])
          // Build the transaction
          .buildUnsignedTransaction();

      // Build the transaction request
      const submitTransactionRequest = buildTransactionRequest(
        transaction,
        account.account_id,
        true
      );

      // Submit the transaction and wait for the result
      const txResult = await submitAndWaitForTransaction(signer, submitTransactionRequest);
      // TODO: this needs improvement
      const pair = txResult.newComponents.find(([_id, state]) => {
        const substate = state.substate;
        console.log({_id, substate});
        if (!substate || !("Component" in substate)) {
          return false;
        }

        // FIXME: template_address is a string not a Buffer
        return (substate.Component! as ComponentHeader).template_address as unknown as string === TEMPLATE_ADDRESS;
      });
      console.log(pair);
      if (!pair) {
        setErrorMessage("Failed to increment counter. Component not found in transaction result.");
        return;
      }
      const id = substateIdToString(pair[0]); // The component address
      setCounterComponentAddress(id);
      // @ts-ignore
      const component = pair[1].substate.Component as ComponentHeader;
      const updatedValue = getCborValueByPath(component.body.state, "$.value") as bigint;
      console.log("Updated Value:", updatedValue);
      setValue(updatedValue);
      setTxResult(txResult);  // Save the transaction result
    } catch (error) {
      console.error("Transaction error:", error);
      setErrorMessage("Failed to submit the transaction.");
    } finally {
      setIsSubmitting(false);
    }
  };

  const listSubstates = async () => {
    if (!signer) {
      setErrorMessage("Signer is not available. Please connect to the wallet first.");
      return;
    }

    try {
      const response = await signer
          .listSubstates({filter_by_template: TEMPLATE_ADDRESS, filter_by_type: null, limit: 10, offset: 0});
      setSubstates(response.substates);
    } catch (error) {
      console.error("Error fetching substates:", error);
      setErrorMessage("Failed to fetch substates.");
    }
  };

  useEffect(() => {
    if (showSubstates) {
      listSubstates();
    }
  }, [showSubstates]);

  const incrementCounterByAddress = async () => {
    if (!signer || !counterComponentAddress) {
      setErrorMessage("Signer or substate address is not available. Please enter a valid substate address.");
      return;
    }

    setErrorMessage(null);
    setIsSubmitting(true);
    try {

      // Get the account executing the transaction
      const account = await signer.getAccount();
      let builder = new TransactionBuilder(NetworkByte.Igor);

      const transaction =
       builder
           .feeTransactionPayFromComponent(account.address, fee.toString())
           .callMethod({
        componentAddress: counterComponentAddress,
        methodName: "increase", // Call the increase method
      }, [])
     .buildUnsignedTransaction();


      const submitTransactionRequest = buildTransactionRequest(
        transaction,
       account.account_id,
        true
      );
      const txResult = await submitAndWaitForTransaction(signer, submitTransactionRequest);

      console.log("Increment Transaction Result:", txResult);
      // TODO: this needs improvement
      const pair = txResult.newComponents.find(([_id, state]) => {
        const substate = state.substate;
        if (!substate || !("Component" in substate)) {
          return false;
        }
        // FIXME: template_address is a string not a Buffer
        return (substate.Component! as ComponentHeader).template_address as unknown as string === TEMPLATE_ADDRESS;
      });
      if (!pair) {
        setErrorMessage("Failed to increment counter. Component not found in transaction result.");
        return;
      }
      const id = substateIdToString(pair[0]); // The component address
      setCounterComponentAddress(id);
      // @ts-ignore
      const component = pair[1].substate.Component as ComponentHeader;
      const updatedValue = getCborValueByPath(component.body.state, "$.value") as bigint;
      console.log("Updated Value:", updatedValue);
      setValue(updatedValue);
    } catch (error) {
      console.error("Error incrementing counter:", error);
      setErrorMessage("Failed to increment counter.");
    } finally {
      setIsSubmitting(false);
    }
  };


  return (
    <>
      <div>
        <a href="https://vite.dev" target="_blank">
          <img src={viteLogo} className="logo" alt="Vite logo" />
        </a>
        <a href="https://react.dev" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>
      <h1>Counter example</h1>
      {errorMessage && (
        <div className="error-message">
          <p>Error: {errorMessage}</p>
        </div>
      )}

      {/* Display the Connection Button and Connection status */}
  <>
      <TariConnectButton
        isConnected={signer?.isConnected() || false}
        walletConnectParams={wcParams}
        walletDaemonParams={{
          serverUrl: "http://localhost:9000/json_rpc", // Replace with your Tari Wallet Daemon URL
          permissions: (new TariPermissions()).addPermission("Admin")
        }}
        onConnected={onConnected}
      />
      {account ? (
        <div>
          <h2>Connected Account</h2>
          <p>Account ID: {account.account_id}</p>
          <p>Address: {account.address}</p>
          <p>Public Key: {account.public_key}</p>
          <h3>Resources:</h3>
          <ul>
            {account.resources.map((resource, index) => (
              <li key={index}>
                {resource.type} - {resource.balance} {resource.token_symbol}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </>

        {/* Transaction Submit Button */}
        <button onClick={createCounter} disabled={isSubmitting} className="submit-button">
          {isSubmitting ? "Submitting Create Counter Transaction..." : "Create Counter"}
        </button>

      <div>
          <h3>Increment Counter</h3>
          <button
            onClick={incrementCounterByAddress}
            disabled={isSubmitting || !counterComponentAddress}
            className="increment-button"
          >
            {isSubmitting ? "Incrementing..." : "Increment Counter"}
          </button>
        </div>


        {/* List Substates Button */}
        <button onClick={() => setShowSubstates(!showSubstates)} className="list-substates-button">
          List Substates
        </button>

        {/* Display Substates */}
        {showSubstates && (
          <div>
            <h3>Substates:</h3>
            <ul>
              {substates?.map((substate, index) => (
                <li key={index}>
                  {JSON.stringify(substate, null, 2)}
                </li>
              )) || "Loading substates..."}
            </ul>
          </div>
        )}


       {/* Display Transaction Result */}
       {txResult && (
          <div>
            <h3>Transaction Result:</h3>
            <p>Counter Created</p>
            <p>Component Address: {counterComponentAddress || '--'}</p>
            <p>Value: {value === null ? '--' : value}</p>
            

            {/* Toggle Button for Full JSON */}
            <button onClick={() => setShowFullJson(!showFullJson)} className="toggle-json-button">
              {showFullJson ? "Hide Full JSON" : "Show Full JSON"}
            </button>

            {/* Collapsible JSON Section */}
            {showFullJson && (
              <pre style={{ padding: "10px", borderRadius: "5px" }}>
                {JSON.stringify(txResult, null, 2)}
              </pre>
            )}
          </div>
        )}

    </>
  );
}

export default App;