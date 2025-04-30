use std::str::FromStr;

use actions::NearAction;
use ed25519_dalek::{PublicKey as PublicKey2, Signature as Ed25519Signature2, Verifier};
use hex::FromHex;
use near_gas::NearGas;
use near_sdk::base64;
use near_sdk::collections::UnorderedSet;
use near_sdk::ext_contract;
use near_sdk::json_types::{Base58CryptoHash, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    bs58, env, near, AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseError, PublicKey,
};
use omni_transaction::TransactionBuilder;
use omni_transaction::TxBuilder;
use omni_transaction::{
    near::types::{
        Action as OmniAction, BlockHash as OmniBlockHash,
        FunctionCallAction as OmniFunctionCallAction, Secp256K1Signature, Signature,
        U128 as OmniU128, U64 as OmniU64,
    },
    NEAR,
};

pub use crate::models::*;
pub use crate::serializer::SafeU128;

mod actions;
mod models;
mod serializer;
mod utils;

#[ext_contract(ext_self)]
pub trait ExtSelf {
    fn callback_method(&mut self, #[callback_result] call_result: Result<Vec<u8>, PromiseError>);
}

const GAS_FOR_REQUEST_SIGNATURE: Gas = Gas::from_tgas(100);
const BASE_GAS: Gas = Gas::from_tgas(10); // Base gas for contract execution
const CALLBACK_GAS: Gas = Gas::from_tgas(10); // Gas reserved for callback

const TESTNET_SIGNER: &str = "v1.signer-prod.testnet";
const MAINNET_SIGNER: &str = "v1.signer";

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ActionString {
    FunctionCall {
        method_name: String,
        args: serde_json::Value,
        gas: String,
        deposit: String,
    },
    Transfer {
        deposit: String,
    },
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct ProxyContract {
    owner_id: AccountId,
    authorized_users: UnorderedSet<AccountId>,
    signer_contract: AccountId,
}

#[near]
impl ProxyContract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Contract is already initialized");

        // Determine signer contract based on current network
        let binding = env::current_account_id();
        let current_network = binding.as_str().split('.').last().unwrap_or("testnet");

        let signer_contract = match current_network {
            "near" => MAINNET_SIGNER,
            _ => TESTNET_SIGNER,
        };

        Self {
            owner_id,
            authorized_users: UnorderedSet::new(b"a"),
            signer_contract: signer_contract.parse().unwrap(),
        }
    }

    pub fn set_signer_contract(&mut self, new_signer: AccountId) {
        self.assert_owner();
        self.signer_contract = new_signer;
    }

    pub fn get_signer_contract(&self) -> AccountId {
        self.signer_contract.clone()
    }

    pub fn get_owner_id(&self) -> AccountId {
        self.owner_id.clone()
    }

    #[payable]
    // TODO - test this when ed25519 signatuers are live and add a callback fn specifically for this
    pub fn request_sign_message(
        &mut self,
        contract_id: AccountId,
        method_name: Option<String>,
        args: String,
        gas: U64,
        deposit: NearToken,
        nonce: U64,
        block_hash: Base58CryptoHash,
        account_pk_for_mpc: String,
    ) -> Promise {
        let attached_gas = env::prepaid_gas();
        assert!(
            attached_gas >= GAS_FOR_REQUEST_SIGNATURE,
            "Not enough gas attached. Please attach at least {} TGas. Attached: {} TGas",
            GAS_FOR_REQUEST_SIGNATURE.as_tgas(),
            attached_gas.as_tgas()
        );

        assert!(
            self.authorized_users
                .contains(&env::predecessor_account_id()),
            "Unauthorized: only authorized users can request signatures"
        );

        // Calculate remaining gas after base costs
        let remaining_gas = attached_gas.saturating_sub(BASE_GAS);
        let gas_for_signing = remaining_gas.saturating_sub(CALLBACK_GAS);

        near_sdk::env::log_str(&format!(
            "Request received - Contract: {}, Method: {:?}, Args: {}, Gas: {}, Deposit: {}, Nonce: {}, Block Hash: {:?}",
            contract_id,
            method_name,
            args,
            gas.0,
            deposit.as_yoctonear(),
            nonce.0,
            block_hash
        ));

        let action = NearAction {
            method_name: method_name.clone(),
            contract_id: contract_id.clone(),
            gas_attached: NearGas::from_gas(gas.0),
            deposit_attached: deposit,
        };

        // verify the action is permitted
        NearAction::is_allowed(&action);

        // Store original args for callback verification
        let original_args = args.clone();
        near_sdk::env::log_str(&format!("Original args: {}", original_args));

        // Extract just the function call args as the payload
        let args_bytes = args.into_bytes();
        let hashed_payload = utils::hash_payload(&args_bytes);

        near_sdk::env::log_str(&format!(
            "Signing just args payload - Hash: {:?}",
            bs58::encode(&hashed_payload).into_string()
        ));

        let request = SignRequest {
            payload: args_bytes, // Send raw args instead of transaction hash
            path: account_pk_for_mpc,
            key_version: 0,
        };

        let request_payload = serde_json::json!({
            "request": request,
        });

        Promise::new(self.signer_contract.clone())
            .function_call(
                "sign".to_string(),
                near_sdk::serde_json::to_vec(&request_payload).unwrap(),
                env::attached_deposit(),
                gas_for_signing,
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(CALLBACK_GAS)
                    .sign_request_callback(original_args),
            )
    }

    #[payable]
    pub fn request_signature(
        &mut self,
        contract_id: AccountId,
        actions_json: String,
        nonce: U64,
        block_hash: Base58CryptoHash,
        mpc_signer_pk: String,
        account_pk_for_mpc: String,
    ) -> Promise {
        let attached_gas = env::prepaid_gas();
        assert!(
            attached_gas >= GAS_FOR_REQUEST_SIGNATURE,
            "Not enough gas attached. Please attach at least {} TGas. Attached: {} TGas",
            GAS_FOR_REQUEST_SIGNATURE.as_tgas(),
            attached_gas.as_tgas()
        );

        assert!(
            self.authorized_users
                .contains(&env::predecessor_account_id()),
            "Unauthorized: only authorized users can request signatures"
        );

        // Calculate remaining gas after base costs
        let remaining_gas = attached_gas.saturating_sub(BASE_GAS);
        let gas_for_signing = remaining_gas.saturating_sub(CALLBACK_GAS);

        // Parse actions from JSON string
        let actions: Vec<ActionString> = serde_json::from_str(&actions_json)
            .unwrap_or_else(|e| panic!("Failed to parse actions JSON: {:?}", e));

        near_sdk::env::log_str(&format!(
            "Request received - Contract: {}, Actions: {:?}, Nonce: {}, Block Hash: {:?}",
            contract_id, actions, nonce.0, block_hash
        ));

        // Convert string actions to OmniActions
        let omni_actions: Vec<OmniAction> = actions
            .into_iter()
            .map(|action| match action {
                ActionString::FunctionCall {
                    method_name,
                    args,
                    gas,
                    deposit,
                } => {
                    let gas_u64 = U64::from(gas.parse::<u64>().unwrap());
                    let deposit_near = NearToken::from_yoctonear(deposit.parse().unwrap());
                    let safe_deposit = SafeU128(deposit_near.as_yoctonear());

                    // Verify action is allowed
                    let near_action = NearAction {
                        method_name: Some(method_name.clone()),
                        contract_id: contract_id.clone(),
                        gas_attached: NearGas::from_gas(gas_u64.0),
                        deposit_attached: deposit_near,
                    };
                    NearAction::is_allowed(&near_action);

                    // Convert args to bytes
                    let args_bytes = serde_json::to_vec(&args)
                        .unwrap_or_else(|e| panic!("Failed to serialize args: {:?}", e));

                    OmniAction::FunctionCall(Box::new(OmniFunctionCallAction {
                        method_name,
                        args: args_bytes,
                        gas: OmniU64(gas_u64.into()),
                        deposit: safe_deposit.0.into(),
                    }))
                }
                ActionString::Transfer { deposit } => {
                    let deposit_near = NearToken::from_yoctonear(deposit.parse().unwrap());
                    let safe_deposit = SafeU128(deposit_near.as_yoctonear());
                    OmniAction::Transfer(omni_transaction::near::types::TransferAction {
                        deposit: safe_deposit.0.into(),
                    })
                }
            })
            .collect();

        // construct the entire transaction to be signed
        let tx = TransactionBuilder::new::<NEAR>()
            .signer_id(env::current_account_id().to_string())
            .signer_public_key(utils::convert_pk_to_omni(
                &PublicKey::from_str(&mpc_signer_pk).unwrap(),
            ))
            .nonce(nonce.0) // Use the provided nonce
            .receiver_id(contract_id.to_string())
            .block_hash(OmniBlockHash(block_hash.into()))
            .actions(omni_actions.clone())
            .build();

        // Extract deposit values from omni_actions
        let deposits: Vec<OmniU128> = omni_actions
            .iter()
            .map(|action| match action {
                OmniAction::FunctionCall(call) => call.deposit.clone(),
                OmniAction::Transfer(transfer) => transfer.deposit.clone(),
                _ => OmniU128(0),
            })
            .collect();

        near_sdk::env::log_str(&format!("Action deposits: {:?}", deposits));

        // Serialize transaction into a string to pass into callback
        let mut tx_json_string = serde_json::to_string(&tx)
            .unwrap_or_else(|e| panic!("Failed to serialize NearTransaction: {:?}", e));

        // Convert any large deposit numbers to strings in the JSON
        let modified_tx_string = deposits.iter().fold(tx_json_string, |acc, deposit| {
            acc.replace(
                &format!("\"deposit\":{}", deposit.0.to_string()),
                &format!("\"deposit\":\"{}\"", deposit.0.to_string()),
            )
        });
        tx_json_string = modified_tx_string;
        near_sdk::env::log_str(&format!("near tx in json: {}", tx_json_string));

        near_sdk::env::log_str(&format!(
            "Transaction details - Receiver: {}, Signer: {}, Actions: {:?}, Nonce: {}, BlockHash: {:?}",
            contract_id,
            env::current_account_id(),
            omni_actions,
            nonce.0,
            block_hash
        ));

        // helpful references
        // https://github.com/PiVortex/subscription-example/blob/main/contract/src/charge_subscription.rs#L129
        //https://github.com/near/near-api-js/blob/a33274d9c06fec7de756f4490dea0618b2fc75da/packages/transactions/src/sign.ts#L39
        //https://github.com/near/near-api-js/blob/master/packages/transactions/src/signature.ts#L21
        //https://github.com/near/near-api-js/blob/a33274d9c06fec7de756f4490dea0618b2fc75da/packages/providers/src/json-rpc-provider.ts#L112C32-L112C49

        // SHA-256 hash of the serialized transaction
        let hashed_payload = utils::hash_payload(&tx.build_for_signing());

        // Log arguments used for signature request
        near_sdk::env::log_str(&format!(
            "Signing request - transaction hash: {:?}, Path: {}, Key Version: {}",
            bs58::encode(&hashed_payload).into_string(),
            account_pk_for_mpc,
            0
        ));
        // Create a signature request for the hashed payload
        let request = SignRequest {
            payload: hashed_payload.to_vec(),
            path: account_pk_for_mpc,
            key_version: 0,
        };

        let request_payload = serde_json::json!({ "request": request });

        // Call MPC requesting a signature for the above txn
        Promise::new(self.signer_contract.clone())
            .function_call(
                "sign".to_string(),
                near_sdk::serde_json::to_vec(&request_payload).unwrap(),
                env::attached_deposit(),
                gas_for_signing,
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(CALLBACK_GAS)
                    .sign_request_callback(tx_json_string),
            )
    }

    #[private] // Only callable by the contract itself
    pub fn sign_request_callback(
        &mut self,
        #[callback_result] call_result: Result<SignatureResponse, PromiseError>,
        tx_json_string: String,
    ) -> String {
        let response = match call_result {
            Ok(json) => {
                near_sdk::env::log_str(&format!("Parsed JSON response: {:?}", json));
                json
            }
            Err(e) => {
                near_sdk::env::log_str(&format!("Failed to parse JSON: {:?}", e));
                panic!("Failed to parse response JSON");
            }
        };

        // Big R value from the MPC signature
        let big_r = response.big_r.affine_point;
        let scalar = response.s.scalar;
        let recovery_id = response.recovery_id;
        near_sdk::env::log_str(&format!("R value: {}", big_r));
        near_sdk::env::log_str(&format!("S value: {}", scalar));
        near_sdk::env::log_str(&format!("Recovery ID value: {}", recovery_id));

        // Split big r into its parts
        let r = &big_r[2..];
        let end = &big_r[..2];

        // Convert hex to bytes
        let r_bytes = Vec::from_hex(r).expect("Invalid hex in r");
        let s_bytes = Vec::from_hex(scalar).expect("Invalid hex in s");
        let end_bytes = Vec::from_hex(end).expect("Invalid hex in end");

        // Add individual bytes together in the correct order
        let mut signature_bytes = [0u8; 65];
        signature_bytes[..32].copy_from_slice(&r_bytes);
        signature_bytes[32..64].copy_from_slice(&s_bytes);
        signature_bytes[64] = end_bytes[0];

        // Create signature
        let omni_signature = Signature::SECP256K1(Secp256K1Signature(signature_bytes));

        // Log signature bytes
        near_sdk::env::log_str(&format!("Signature bytes: {:?}", &signature_bytes));

        // Deserialize transaction
        let near_tx = serde_json::from_str::<models::NearTransaction>(&tx_json_string)
            .unwrap_or_else(|_| panic!("Failed to deserialize transaction: {:?}", tx_json_string));

        // Log signature in hex format
        near_sdk::env::log_str(&format!(
            "Signature in hex: {:?}",
            hex::encode(&signature_bytes)
        ));

        // Log signature in base58 format
        let siggy_base58 = bs58::encode(&signature_bytes).into_string();
        near_sdk::env::log_str(&format!("Signature in base58: {}", siggy_base58));

        // Add signature to transaction
        let near_tx_signed = near_tx.build_with_signature(omni_signature);

        let base64_tx =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &near_tx_signed);
        near_sdk::env::log_str(&format!("Signed transaction (base64): {}", base64_tx));

        base64_tx
    }

    pub fn test_recover(&self, hash: Vec<u8>, signature: Vec<u8>, v: u8) -> Option<String> {
        let recovered: Option<[u8; 64]> = env::ecrecover(&hash, &signature, v, true);

        env::log_str(&format!("Hash: {}", hex::encode(&hash)));
        env::log_str(&format!("Signature: {}", hex::encode(&signature)));
        env::log_str(&format!("V: {}", v));

        recovered.map(|key: [u8; 64]| {
            let hex_key: String = hex::encode(&key);
            env::log_str(&format!("Recovered key: {}", hex_key));
            hex_key
        })
    }

    pub fn verify_ed25519_signature(
        &self,
        message: Vec<u8>,
        signature: Vec<u8>,
        public_key: Vec<u8>,
    ) -> bool {
        env::log_str(&format!("Message: {}", hex::encode(&message)));
        env::log_str(&format!("Signature: {}", hex::encode(&signature)));
        env::log_str(&format!("Public Key: {}", hex::encode(&public_key)));

        match (
            Ed25519Signature2::from_bytes(signature.as_slice().try_into().unwrap()),
            PublicKey2::from_bytes(public_key.as_slice().try_into().unwrap()),
        ) {
            (Ok(sig), Ok(pk)) => match pk.verify(&message, &sig) {
                Ok(_) => {
                    env::log_str("Signature verification succeeded");
                    true
                }
                Err(e) => {
                    env::log_str(&format!("Verification failed: {:?}", e));
                    false
                }
            },
            _ => {
                env::log_str("Failed to parse signature or public key");
                false
            }
        }
    }

    // Owner methods for managing authorized users
    pub fn add_authorized_user(&mut self, account_id: AccountId) {
        self.assert_owner();
        self.authorized_users.insert(&account_id);
    }

    pub fn remove_authorized_user(&mut self, account_id: AccountId) {
        self.assert_owner();
        self.authorized_users.remove(&account_id);
    }

    pub fn get_authorized_users(&self) -> Vec<AccountId> {
        self.authorized_users.to_vec()
    }

    // View methods
    pub fn is_authorized(&self, account_id: AccountId) -> bool {
        self.authorized_users.contains(&account_id) || self.owner_id == account_id
    }

    // Helper methods
    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "Be gone. You have no power here."
        );
    }
}
