import { createContext } from 'react';
import {
  addFunctionCallAccessKey,
  generateRandomKeyPair,
  getSignerFromKeystore,
  getTestnetRpcProvider,
} from '@near-js/client';

import { BrowserLocalStorageKeyStore } from '@near-js/keystores-browser';

// near api js
import { providers, utils, KeyPair } from 'near-api-js';

// wallet selector
import '@near-wallet-selector/modal-ui/styles.css';
import { setupModal } from '@near-wallet-selector/modal-ui';
import { setupWalletSelector } from '@near-wallet-selector/core';
import { setupBitteWallet } from '@near-wallet-selector/bitte-wallet';

import crypto from 'crypto';

import { getQuotes } from '@/utils/getQuotes';
const THIRTY_TGAS = '30000000000000';
const NO_DEPOSIT = '0';

export class Wallet {
  /**
   * @constructor
   * @param {Object} options - the options for the wallet
   * @param {string} options.networkId - the network id to connect to
   * @param {string} options.createAccessKeyFor - the contract to create an access key for
   * @example
   * const wallet = new Wallet({ networkId: 'testnet', createAccessKeyFor: 'contractId' });
   * wallet.startUp((signedAccountId) => console.log(signedAccountId));
   */
  constructor({ networkId = 'testnet', createAccessKeyFor = undefined }) {
    this.createAccessKeyFor = createAccessKeyFor;
    this.networkId = networkId;
    // Expose intents for deposit, swap, withdraw, and public key check
    this.intents = {
      deposit: this.depositIntent.bind(this),
      swap: this.swapIntent.bind(this),
      withdraw: this.withdrawIntent.bind(this),
      hasPublicKey: this.hasPublicKey.bind(this),
    };
  }

  /**
   * To be called when the website loads
   * @param {Function} accountChangeHook - a function that is called when the user signs in or out#
   * @returns {Promise<string>} - the accountId of the signed-in user 
   */
  startUp = async (accountChangeHook) => {
    this.selector = setupWalletSelector({
      network: this.networkId,
      modules: [
        setupBitteWallet(),
      ],
    });

    const walletSelector = await this.selector;
    const isSignedIn = walletSelector.isSignedIn();
    const accountId = isSignedIn ? walletSelector.store.getState().accounts[0].accountId : '';
    // Store the signed-in account ID for later use in other methods.
    this.signedAccountId = accountId;

    walletSelector.store.observable.subscribe(async (state) => {
      const signedAccount = state?.accounts.find(account => account.active)?.accountId;
      this.signedAccountId = signedAccount || '';
      accountChangeHook(signedAccount || '');
    });

    return accountId;
  };

  /**
   * Displays a modal to login the user
   */
  signIn = async () => {
    const modal = setupModal(await this.selector, { contractId: this.createAccessKeyFor });
    modal.show();
  };

  /**
   * Logout the user
   */
  signOut = async () => {
    const selectedWallet = await (await this.selector).wallet();
    selectedWallet.signOut();
  };

  /**
   * Makes a read-only call to a contract
   * @param {Object} options - the options for the call
   * @param {string} options.contractId - the contract's account id
   * @param {string} options.method - the method to call
   * @param {Object} options.args - the arguments to pass to the method
   * @returns {Promise<JSON.value>} - the result of the method call
   */
  viewMethod = async ({ contractId, method, args = {} }) => {
    const url = `https://rpc.${this.networkId}.near.org`;
    const provider = new providers.JsonRpcProvider({ url });

    const encodedArgs = Buffer.from(JSON.stringify(args)).toString('base64');
    
    try {
      const res = await provider.query({
        request_type: 'call_function',
        account_id: contractId,
        method_name: method,
        args_base64: encodedArgs,
        finality: 'optimistic',
      });
      return JSON.parse(Buffer.from(res.result).toString());
    } catch (error) {
      console.error("Error querying NEAR view method:", error);
      throw error;
    }
  };
  /**
   * Makes a call to a contract
   * @param {Object} options - the options for the call
   * @param {string} options.contractId - the contract's account id
   * @param {string} options.method - the method to call
   * @param {Object} options.args - the arguments to pass to the method
   * @param {string} options.gas - the amount of gas to use
   * @param {string} options.deposit - the amount of yoctoNEAR to deposit
   * @returns {Promise<Transaction>} - the resulting transaction
   */
  callMethod = async ({ contractId, method, args = {}, gas = THIRTY_TGAS, deposit = NO_DEPOSIT }) => {
    // Sign a transaction with the "FunctionCall" action
    const selectedWallet = await (await this.selector).wallet();
    const outcome = await selectedWallet.signAndSendTransaction({
      receiverId: contractId,
      actions: [
        {
          type: 'FunctionCall',
          params: {
            methodName: method,
            args,
            gas,
            deposit,
          },
        },
      ],
    });

    return providers.getTransactionLastResult(outcome);
  };

  /**
   * Makes a call to a contract
   * @param {string} txhash - the transaction hash
   * @returns {Promise<JSON.value>} - the result of the transaction
   */
  getTransactionResult = async (txhash) => {
    const walletSelector = await this.selector;
    const { network } = walletSelector.options;
    const provider = new providers.JsonRpcProvider({ url: network.nodeUrl });

    // Retrieve transaction result from the network
    const transaction = await provider.txStatus(txhash, 'unnused');
    return providers.getTransactionLastResult(transaction);
  };

  /**
   * Gets the balance of an account
   * @param {string} accountId - the account id to get the balance of
   * @returns {Promise<number>} - the balance of the account
   *  
   */
  getBalance = async (accountId) => {
    const walletSelector = await this.selector;
    const { network } = walletSelector.options;
    const provider = new providers.JsonRpcProvider({ url: network.nodeUrl });

    // Retrieve account state from the network
    const account = await provider.query({
      request_type: 'view_account',
      account_id: accountId,
      finality: 'final',
    });
    // return amount on NEAR
    return account.amount ? Number(utils.format.formatNearAmount(account.amount)) : 0;
  };

  /**
   * Signs and sends transactions
   * @param {Object[]} transactions - the transactions to sign and send
   * @returns {Promise<Transaction[]>} - the resulting transactions
   * 
   */
  signAndSendTransactions = async ({ transactions }) => {
    const selectedWallet = await (await this.selector).wallet();
    return selectedWallet.signAndSendTransactions({ transactions });
  };

  /**
   * 
   * @param {string} accountId
   * @returns {Promise<Object[]>} - the access keys for the
   */
  getAccessKeys = async (accountId) => {
    const walletSelector = await this.selector;
    const { network } = walletSelector.options;
    const provider = new providers.JsonRpcProvider({ url: network.nodeUrl });

    // Retrieve account state from the network
    const keys = await provider.query({
      request_type: 'view_access_key_list',
      account_id: accountId,
      finality: 'final',
    });
    return keys.keys;
  };

  /**
   * Registers the user's wallet with the contract.
   * NOTE: Adjust the contractId and parameters as required for your application.
   */
  register = async (key) => {
    try {
      const result = await this.callMethod({
        contractId: "intents.near", // Placeholder contract for registration
        method: "add_public_key",
        args: { public_key: key },
        gas: THIRTY_TGAS,
        deposit: "1"
      });
    } catch (error) {
      console.error("Register failed:", error);
    }
  };

  // New deposit intent method
  depositIntent = async (amount) => {
    try {
      const depositAmount = utils.format.parseNearAmount(amount);
      const nearDepositAction = {
        type: 'FunctionCall',
        params: {
          methodName: 'near_deposit',
          args: {},
          gas: THIRTY_TGAS,
          deposit: depositAmount,
        }
      };
      const ftTransferCallAction = {
        type: 'FunctionCall',
        params: {
          methodName: 'ft_transfer_call',
          args: {
            receiver_id: "intents.near",
            amount: depositAmount,
            msg: ""
          },
          gas: THIRTY_TGAS,
          deposit: "1",
        }
      };
      const transactions = [{
        receiverId: "wrap.near",
        actions: [nearDepositAction, ftTransferCallAction]
      }];
      const result = await this.signAndSendTransactions({ transactions });
      return result;
    } catch (error) {
      console.error("Deposit intent failed:", error);
      return { error: error.toString() };
    }
  };

  swapIntent = async (amount, quoteData, deadlineDeltaMs = 60000) => {
    try {
      // Convert provided NEAR amount to yoctoNEAR.
      const nearAmountYocto = utils.format.parseNearAmount(amount);
      if (!nearAmountYocto) {
        throw new Error("Invalid NEAR amount provided.");
      }

      // Define token identifiers.
      const tokenInId = "nep141:wrap.near";
      const assetIdentifierOut = "nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1";

      // Retrieve the best quote.
      const { bestQuote } = await getQuotes(tokenInId, nearAmountYocto, assetIdentifierOut);
      if (!bestQuote) {
        throw new Error("No best quote returned from getQuotes");
      }

      // Calculate referral fee (1% of amount_out) and net output.
      const referralFeeAmount = String(Math.floor(parseInt(bestQuote.amount_out) / 100));
      const amountOutLessFee = String(parseInt(bestQuote.amount_out) - parseInt(referralFeeAmount));

      // Set a deadline (e.g., 60 seconds from now).
      const deadline = new Date(Date.now() + deadlineDeltaMs).toISOString();

      // Generate a random nonce (32 bytes).
      const nonce = crypto.randomBytes(32);

      // Construct the payload matching the Python reference.
      const payload = {
        signer_id: this.signedAccountId,
        nonce: nonce,
        verifying_contract: "intents.near",
        deadline: deadline,
        intents: [
          {
            intent: "token_diff",
            diff: {
              [bestQuote.token_in]: "-" + bestQuote.amount_in,
              [bestQuote.token_out]: amountOutLessFee,
            },
            referral: "benevio-labs.near"
          },
          {
            intent: "transfer",
            receiver_id: "benevio-labs.near",
            tokens: {
              [bestQuote.token_out]: referralFeeAmount
            },
            memo: "referral_fee"
          }
        ]
      };

      // Ensure the user is signed in.
      if (!this.signedAccountId) {
        throw new Error("Wallet is not signed in");
      }

      // Prepare the message for signing.
      const recipient = "intents.near";
      const msg = {
        message: JSON.stringify(payload),
        nonce: nonce,
        recipient: recipient,
        callbackUrl: undefined
      };

      // Get the wallet and sign the message.
      const walletSelector = await this.selector;
      const selectedWallet = await walletSelector.wallet();
      const signedPayload = await selectedWallet.signMessage(msg);

      // Publish the signed intent via the RPC endpoint.
      const rpcResponse = await fetch("https://solver-relay-v2.chaindefuser.com/rpc", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          id: "dontcare",
          jsonrpc: "2.0",
          method: "publish_intent",
          params: signedPayload
        })
      });

      const published = await rpcResponse.json();
      return published;
    } catch (error) {
      console.error("Swap intent failed:", error);
      return { error: error.toString() };
    }
  };

  withdrawIntent = async (amount, deadlineDeltaMs = 60000) => {
    try {
      const receiver = this.signedAccountId || "unknown";
      // Build the ft_withdraw intent
      const ftWithdrawIntent = {
         intent: "ft_withdraw",
         token: "17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1",
         receiver_id: receiver,
         amount: amount,
         signer_id: receiver
      };

      // Build the USDC_SWAP_INTENT with a parameterized deadline and token diffs.
      const deadline = new Date(Date.now() + deadlineDeltaMs).toISOString();
      const usdcSwapIntent = {
         deadline: deadline,
         intents: [
           {
             intent: "token_diff",
             diff: {
               "nep141:eth-0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.omft.near": "-" + amount,
               "nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1": amount
             },
             referral: "near-intents.intents-referral.near"
           },
         ]
      };

      // Combine both intents into a composite payload
      const combinedIntent = [ftWithdrawIntent, usdcSwapIntent];

      const rpcResponse = await fetch("https://solver-relay-v2.chaindefuser.com/rpc", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
              id: "dontcare",
              jsonrpc: "2.0",
              method: "publish_intent",
              params: combinedIntent
          })
      });
      const published = await rpcResponse.json();
      return published;
    } catch (error) {
      console.error("Withdraw intent failed:", error);
      return { error: error.toString() };
    }
  };

  /**
   * Checks if the current account's public key is registered on NEAR.
   * @param {string} key - The public key to check.
   * @returns {Promise<boolean>} - True if the public key is registered, false otherwise.
   */
  hasPublicKey = async (key) => {
    if (!this.signedAccountId) return false;
    try {
      const result = await this.viewMethod({
        contractId: "intents.near",
        method: "has_public_key",
        args: { account_id: this.signedAccountId, public_key: key },
      });
      return result;
    } catch (error) {
      console.error("Error checking public key registration:", error);
      return false;
    }
  };

  /**
   * Creates a new FunctionCall access key for the signed-in user.
   * Outputs the private key to the console for testing.
   * Returns the new key pair.
   */

  createFunctionKey = async () => {
    if (!this.signedAccountId) {
      console.error("User not signed in");
      return;
    }
    const accountId = this.signedAccountId;
    
    // Initialize RPC provider (assuming testnet; adjust if using another network)
    const rpcProvider = getTestnetRpcProvider();
    
    // Initialize the signer using the wallet store's credentials (assumes credentials exist on the local file system)
    const walletSelector = await this.selector;
    const keystore = new BrowserLocalStorageKeyStore();
    const signer = getSignerFromKeystore(
      accountId,
      this.networkId,
      keystore
    );
    console.log("await", (await signer).getPublicKey());
    
    // Generate a new key pair using random data
    const keyPair = generateRandomKeyPair('ed25519');
    const publicKey = keyPair.getPublicKey().toString();
    const privateKey = keyPair.secretKey;
    
    // Add the generated key as a Function Call Access Key
    await addFunctionCallAccessKey({
      account: accountId,
      publicKey,
      contract: this.createAccessKeyFor || accountId,
      methodNames: [],
      allowance: 2500000000000n,
      deps: { rpcProvider, signer },
    });
    
    console.log("--------------------------------------------------------");
    console.log("RESULTS: Added new function call access key");
    console.log("--------------------------------------------------------");
    console.log(`New Key | ${publicKey} | ${this.createAccessKeyFor || accountId} | []`);
    console.log("--------------------------------------------------------");
    
    return { publicKey, privateKey };
  };
}

/**
 * @typedef NearContext
 * @property {import('./wallets/near').Wallet} wallet Current wallet
 * @property {string} signedAccountId The AccountId of the signed user
 */

/** @type {import ('react').Context<NearContext>} */
export const NearContext = createContext({
  wallet: undefined,
  signedAccountId: '',
});
