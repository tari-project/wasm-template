import {useEffect, useState} from "react";
import { defaultPermissions, TariConnectButton } from "@tari-project/react-mui-connect-button";
import {
  AccountData,
  TransactionBuilder,
  buildTransactionRequest,
  submitAndWaitForTransaction,
  TariSigner,
  TariPermissions, Amount,
} from "@tari-project/tarijs-all";
import reactLogo from './assets/react.svg';
import viteLogo from '/vite.svg';
import './App.css';
import { NetworkByte } from "@tari-project/typescript-bindings";


/**
 * The address of the counter component template at the time of writing on the Igor network.
 * By the time you try this example, the address may no longer work.
 * Workaround: Deploy the template yourself to testnet using the Tari CLI and use the new address here.
 */
const COUNTER_TEMPLATE_ADDRESS = "229611758ac1474ba6bcb83b6927c0c79e07b8e19350f8427d1dfb271107d0df";

// The max fee that the validators are allowed to charge for these transactions (0.001 XTR)
const MAX_FEE = Amount.of(1000);

interface CounterComponent {
  value: bigint; // The current value of the counter
}

function App() {
  const [errorMessage, setErrorMessage] = useState<string | null>(null); // Track any connection errors
  const [isSubmitting, setIsSubmitting] = useState(false); // Track submission state
  const [txResult, setTxResult] = useState<any>(null); // Store transaction result
  const [showFullJson, setShowFullJson] = useState(false); // Toggle for showing full JSON
  const [substates, setSubstates] = useState<any[]>([]); // Store the list of substates
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
          .feeTransactionPayFromComponent(account.address, MAX_FEE)
          // Call the template function to create a new component
          .callFunction(
            {
              templateAddress: COUNTER_TEMPLATE_ADDRESS,
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
      );

      // Submit the transaction and wait for the result
      const txResult = await submitAndWaitForTransaction(signer, submitTransactionRequest);
      const counter = txResult.getComponentsByTemplateAddress(COUNTER_TEMPLATE_ADDRESS)[0];
      if (!counter) {
        setErrorMessage("Failed to increment counter. Component not found in transaction result.");
        return;
      }
      setCounterComponentAddress(counter.id);
      const updatedValue = counter.decodeBody<{value: bigint}>();
      console.log("Updated Value:", updatedValue);
      setValue(updatedValue.value);
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
          .listSubstates({filter_by_template: COUNTER_TEMPLATE_ADDRESS, filter_by_type: null, limit: 10, offset: 0});
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
           .feeTransactionPayFromComponent(account.address, MAX_FEE)
           .callMethod({
              componentAddress: counterComponentAddress,
              methodName: "increase", // Call the increase method
            }, [])
     .buildUnsignedTransaction();

      const submitTransactionRequest = buildTransactionRequest(
          transaction,
          account.account_id,
      );
      const txResult = await submitAndWaitForTransaction(signer, submitTransactionRequest);

      console.log("Increment Transaction Result:", txResult);

      const counter = txResult.getComponentsByTemplateAddress(COUNTER_TEMPLATE_ADDRESS)[0];
      if (!counter) {
        setErrorMessage("Failed to increment counter. Component not found in transaction result.");
        return;
      }
      setCounterComponentAddress(counter.id);
      // @ts-ignore
      const updatedValue = counter.decodeBody<CounterComponent>();
      console.log("Updated Value:", updatedValue);
      setValue(updatedValue.value);
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
            {account.vaults.map((vault, index) => (
              <li key={index}>
                {vault.type} - {vault.balance} {vault.token_symbol}
              </li>
            ))}
          </ul>
        </div>
      ) : null}

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