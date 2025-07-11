import { useState } from "react";
// import { WalletConnectTariSigner } from "@tari-project/wallet-connect-signer";
import { defaultPermissions, TariConnectButton } from "@tari-project/react-mui-connect-button";
import { AccountData, TariSinger, Amount, TransactionBuilder, buildTransactionRequest, Network, submitAndWaitForTransaction, } from "@tari-project/tarijs-all";
import reactLogo from './assets/react.svg';
import viteLogo from '/vite.svg';
import './App.css';

function App() {
  const [isConnected, setIsConnected] = useState(false); // Track wallet connection state
  const [errorMessage, setErrorMessage] = useState<string | null>(null); // Track any connection errors
  const [accountAddress, setAccountAddress] = useState<string | null>(null); // Store the account address
  const [isSubmitting, setIsSubmitting] = useState(false); // Track submission state
  const [txResult, setTxResult] = useState<any>(null); // Store transaction result
  const [showFullJson, setShowFullJson] = useState(false); // Toggle for showing full JSON
  // const [signer, setSigner] = useState<any>(null); // 
  const [substates, setSubstates] = useState<any[]>([]); // Store the list of substates
  const [showSubstates, setShowSubstates] = useState(false); // Toggle for showing substates
  const [substateAddress, setSubstateAddress] = useState<string>(""); // Store the entered substate address

  
  const WC_PROJECT_ID =  "1825b9dd9c17b5a33063ae91cbc48a6e";
  
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

  const createAndSubmitTransaction = async () => {
    if (!accountAddress) {
      setErrorMessage("Account address is not available.");
      return;
    }

    setIsSubmitting(true);  // Start the transaction submission process
    setErrorMessage(null);

    try {
      // Create the fee amount (e.g., 2000 units)
      const fee = new Amount(20000);

      // Initialize the TransactionBuilder
      let builder = new TransactionBuilder();

      // Get the account executing the transaction
      const account = await signer.getAccount();

      // Specify that the fee will be paid from the account
      builder = builder.feeTransactionPayFromComponent(account.address, fee.getStringValue());

      // Template address for creating a new component
      const templateAddress = "2a0399ad3d53490d4fd4984e89d0d6fcad392c4da795117e1a2c01ffe724574d"; 

      // Call the template function to create a new component
      builder = builder.callFunction(
        {
          templateAddress,
          functionName: "new",  // Function to call for creating the new component
        },
        []  // Parameters to pass to the function
      );

      // Optionally, add a fee instruction (if needed)
      builder = builder.addFeeInstruction({
        CallMethod: {
          component_address: account.address,  // Fee is paid from the account
          method: "pay_fee",  // Method to pay the fee
          args: [fee.getStringValue()],  // The fee amount
        },
      });

      // Build the transaction
      const transaction = builder.build();

      // Build the transaction request
      const isDryRun = false;  // Set to false to execute the transaction
      const network = Network.Igor;  // Network to execute the transaction on
      const requiredSubstates = [];  // No specific substates required

      const submitTransactionRequest = buildTransactionRequest(
        transaction,
        account.account_id,
        requiredSubstates,
        undefined,  // Obsolete inputRefs
        isDryRun,
        network
      );

      // Submit the transaction and wait for the result
      const txResult = await submitAndWaitForTransaction(signer, submitTransactionRequest);

      const response = result.result as { execution_results: { indexed: { value: string}}[]};
      const componentAddress = parseCbor(response.execution_results[0].indexed.value);
      console.log({componentAddress});

      setTxResult(txResult);  // Save the transaction result
      setIsSubmitting(false); // Reset the submitting state

    } catch (error) {
      console.error("Transaction error:", error);
      setErrorMessage("Failed to submit the transaction.");
      setIsSubmitting(false);
    }
  };

  const listSubstates = async () => {
    if (!signer) {
      setErrorMessage("Signer is not available. Please connect to the wallet first.");
      return;
    }

    try {
      const templateAddress = "2a0399ad3d53490d4fd4984e89d0d6fcad392c4da795117e1a2c01ffe724574d"; // Template address
      const response = await signer.listSubstates(templateAddress, null, 10, 0); // Fetch substates
      setSubstates(response.substates || []); // Save the substates
      setShowSubstates(true); // Show the substates section
    } catch (error) {
      console.error("Error fetching substates:", error);
      setErrorMessage("Failed to fetch substates.");
    }
  };

  const incrementCounterByAddress = async () => {
    if (!signer || !substateAddress) {
      setErrorMessage("Signer or substate address is not available. Please enter a valid substate address.");
      return;
    }

    setIsSubmitting(true);
    console.log("1");
    try {
      let builder = new TransactionBuilder();
      builder = builder.callMethod({
        componentAddress: substateAddress,
        methodName: "increase", // Call the increase method
      });
      console.log("2");
      const transaction = builder.build();
      console.log("3");
      const isDryRun = false;
      const network = Network.Igor;

      const submitTransactionRequest = buildTransactionRequest(
        transaction,
        accountAddress!,
        [],
        undefined,
        isDryRun,
        network
      );
      console.log("4");
      const txResult = await submitAndWaitForTransaction(signer, submitTransactionRequest);
      console.log("Increment Transaction Result:", txResult);
      console.log("5");
      // Optionally, fetch the updated value of the counter
      const valueResponse = await signer.callMethod({
        componentAddress: substateAddress,
        methodName: "value",
      });
      const updatedValue = valueResponse.result || 0;

      console.log(`Updated Value for Counter (${substateAddress}):`, updatedValue);
      setErrorMessage(`Counter incremented successfully. Updated value: ${updatedValue}`);
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
      <h1>Vite + React</h1>
    
      {/* Display the Connection Button and Connection status */}
  <>
      <TariConnectButton
        isConnected={signer?.isConnected() || false}
        walletConnectParams={wcParams}
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
        <button onClick={createAndSubmitTransaction} disabled={isSubmitting} className="submit-button">
          {isSubmitting ? "Submitting Create Counter Request..." : "Create Counter"}
        </button>

{/* Input for Substate Address */}
<div>
          <h3>Increment Counter by Substate Address</h3>
          <input
            type="text"
            placeholder="Enter Substate Address"
            value={substateAddress}
            onChange={(e) => setSubstateAddress(e.target.value)}
            className="substate-input"
          />
          <button
            onClick={incrementCounterByAddress}
            disabled={isSubmitting || !substateAddress}
            className="increment-button"
          >
            {isSubmitting ? "Incrementing..." : "Increment Counter"}
          </button>
        </div>


        {/* List Substates Button */}
        <button onClick={listSubstates} className="list-substates-button">
          List Substates
        </button>

        {/* Display Substates */}
        {showSubstates && (
          <div>
            <h3>Substates:</h3>
            <ul>
              {substates.map((substate, index) => (
                <li key={index}>
                  {JSON.stringify(substate, null, 2)}
                </li>
              ))}
            </ul>
          </div>
        )}


 {/* Display Transaction Result */}
 {txResult && (
          <div>
            <h3>Transaction Result:</h3>
            <p>Counter Created</p>
            <p>Component Address: {txResult.result?.component_address || "Unknown Address"}</p>
            

            {/* Toggle Button for Full JSON */}
            <button onClick={() => setShowFullJson(!showFullJson)} className="toggle-json-button">
              {showFullJson ? "Hide Full JSON" : "Show Full JSON"}
            </button>

            {/* Collapsible JSON Section */}
            {showFullJson && (
              <pre style={{ background: "#f4f4f4", padding: "10px", borderRadius: "5px" }}>
                {JSON.stringify(txResult, null, 2)}
              </pre>
            )}
          </div>
        )}

      <p className="read-the-docs">
        Click on the Vite and React logos to learn more
      </p>
    </>
  );
}

export default App;