import React, { useState } from "react";
import { WalletConnectTariProvider } from "@tari-project/wallet-connect-provider";
import {
  Amount,
  TransactionBuilder,
  buildTransactionRequest,
  Network,
  submitAndWaitForTransaction,
} from "@tari-project/tarijs-all";
import type { AccountData, TariSigner } from "@tari-project/tarijs-all";

import reactLogo from './assets/react.svg';
import viteLogo from '/vite.svg';
import './App.css';

function App() {
  const [isConnected, setIsConnected] = useState(false); // Track wallet connection state
  const [errorMessage, setErrorMessage] = useState<string | null>(null); // Track any connection errors
  const [accountAddress, setAccountAddress] = useState<string | null>(null); // Store the account address
  const [isSubmitting, setIsSubmitting] = useState(false); // Track submission state
  const [txResult, setTxResult] = useState<any>(null); // Store transaction result

  const projectId = "1825b9dd9c17b5a33063ae91cbc48a6e";
  const provider = new WalletConnectTariProvider(projectId);

  const connectToWallet = async () => {
    try {
      // Try connecting to the wallet
      await provider.connect();
      setIsConnected(true);

      // Fetch account address after successful connection
      const account = await provider.getAccount();
      setAccountAddress(account.address);
      setErrorMessage(null);
    } catch (error) {
      console.error("Error connecting to wallet:", error);
      setErrorMessage("Failed to connect to wallet.");
      setIsConnected(false);
    }
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
      const fee = new Amount(200000);

      // Initialize the TransactionBuilder
      let builder = new TransactionBuilder();

      // Get the account executing the transaction
      const account = await signer.getAccount();

      // Specify that the fee will be paid from the account
      builder = builder.feeTransactionPayFromComponent(account.address, fee.getStringValue());

      // Template address for creating a new component
      const templateAddress = "4e58528c0ab45e0201c617d6860752e23ca02c331235e8907a61c420b7e6465f"; 

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
      const network = Network.LocalNet;  // Network to execute the transaction on
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
      const txResult = await submitAndWaitForTransaction(provider, submitTransactionRequest);
      setTxResult(txResult);  // Save the transaction result
      setIsSubmitting(false); // Reset the submitting state

    } catch (error) {
      console.error("Transaction error:", error);
      setErrorMessage("Failed to submit the transaction.");
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

      {/* Display the connection status */}
      <div className="card">
        <button onClick={connectToWallet} className="connect-button">
          Connect to Wallet
        </button>
        <p>
          {isConnected ? (
            <span style={{ color: 'green' }}>Wallet is connected!</span>
          ) : (
            <span style={{ color: 'red' }}>Wallet is not connected</span>
          )}
        </p>

        {accountAddress && (
          <p>Account Address: {accountAddress}</p>
        )}

        {errorMessage && (
          <p style={{ color: 'red' }}>{errorMessage}</p>
        )}

        {/* Transaction Submit Button */}
        <button onClick={createAndSubmitTransaction} disabled={isSubmitting} className="submit-button">
          {isSubmitting ? "Submitting..." : "Create Counter"}
        </button>

        {/* Display Transaction Result */}
        {txResult && (
          <div>
            <h3>Transaction Result:</h3>
            <p>Counter Created</p>
            <p>Component Address: {txResult.result?.component_address || "Unknown Address"}</p>
          </div>
        )}
      </div>

      <p className="read-the-docs">
        Click on the Vite and React logos to learn more
      </p>
    </>
  );
}

export default App;