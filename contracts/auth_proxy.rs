use actions::{ActionValidationError, NearAction};
use near_gas::NearGas;
use near_sdk::base64;
use near_sdk::collections::UnorderedSet;

use near_sdk::ext_contract;
use near_sdk::json_types::{Base58CryptoHash, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseError, PublicKey, env, near,
};

use omni_transaction::TransactionBuilder;
use omni_transaction::TxBuilder;
use omni_transaction::near::types::{ED25519Signature, Secp256K1Signature};
use omni_transaction::near::utils::PublicKeyStrExt;
use omni_transaction::{
    NEAR,
    near::types::{
        Action as OmniAction, BlockHash as OmniBlockHash,
        FunctionCallAction as OmniFunctionCallAction, Signature, U64 as OmniU64,
    },
};

use once_cell::sync::Lazy;
static NEAR_INTENTS_ADDRESS: Lazy<AccountId> = Lazy::new(|| "intents.near".parse().unwrap());

pub use crate::models::*;
pub use crate::serializer::SafeU128;

mod actions;
mod integration_tests;
mod models;
mod serializer;
mod unit_tests;
mod utils;

// Constants
const GAS_FOR_REQUEST_SIGNATURE: Gas = Gas::from_tgas(100);
const BASE_GAS: Gas = Gas::from_tgas(10); // Base gas for contract execution
const CALLBACK_GAS: Gas = Gas::from_tgas(10); // Gas reserved for callback
const NEAR_MPC_DOMAIN_ID: u32 = 0;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct AuthProxyContract {
    owner_id: AccountId,
    authorized_users: UnorderedSet<AccountId>,
    signer_id: AccountId,
}

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

#[ext_contract(ext_self)]
pub trait ExtSelf {
    fn callback_method(
        &mut self,
        #[callback_result] call_result: Result<SignatureResponse, PromiseError>,
    );
}

#[near]
impl AuthProxyContract {
    #[init]
    pub fn new(owner_id: AccountId, signer_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Contract is already initialized");

        Self {
            owner_id,
            authorized_users: UnorderedSet::new(b"a"),
            signer_id,
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

    pub fn is_authorized(&self, account_id: AccountId) -> bool {
        self.authorized_users.contains(&account_id) || self.owner_id == account_id
    }

    pub fn get_authorized_users(&self) -> Vec<AccountId> {
        self.authorized_users.to_vec()
    }

    pub fn get_owner_id(&self) -> AccountId {
        self.owner_id.clone()
    }

    // Helper methods
    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "You have no power here. Only the owner can perform this action."
        );
    }

    /// Validate and build OmniActions from ActionString inputs
    fn validate_and_build_actions(
        &self,
        actions: Vec<ActionString>,
        contract_id: &AccountId,
    ) -> Result<Vec<OmniAction>, String> {
        if actions.is_empty() {
            return Err("Actions cannot be empty. At least one action is required.".to_string());
        }

        actions
            .into_iter()
            .map(|action| match action {
                ActionString::FunctionCall {
                    method_name,
                    args,
                    gas,
                    deposit,
                } => {
                    let gas_u64 = U64::from(gas.parse::<u64>().map_err(|_| "Invalid gas format")?);
                    let deposit_near = NearToken::from_yoctonear(
                        deposit.parse().map_err(|_| "Invalid deposit format")?,
                    );
                    let safe_deposit = SafeU128(deposit_near.as_yoctonear());

                    // Verify action is allowed
                    let near_action = NearAction {
                        method_name: Some(method_name.clone()),
                        contract_id: contract_id.clone(),
                        gas_attached: NearGas::from_gas(gas_u64.0),
                        deposit_attached: deposit_near,
                    };
                    near_action.is_allowed().map_err(|e| match e {
                        ActionValidationError::ContractNotAllowed(msg) => msg,
                        ActionValidationError::MethodNotAllowed(msg) => msg,
                    })?;

                    // Convert args to bytes
                    let args_bytes = serde_json::to_vec(&args)
                        .map_err(|e| format!("Failed to serialize args: {}", e))?;

                    Ok(OmniAction::FunctionCall(Box::new(OmniFunctionCallAction {
                        method_name,
                        args: args_bytes,
                        gas: OmniU64(gas_u64.into()),
                        deposit: safe_deposit.0.into(),
                    })))
                }
                ActionString::Transfer { deposit } => {
                    let deposit_near = NearToken::from_yoctonear(
                        deposit.parse().map_err(|_| "Invalid deposit format")?,
                    );
                    let safe_deposit = SafeU128(deposit_near.as_yoctonear());
                    Ok(OmniAction::Transfer(
                        omni_transaction::near::types::TransferAction {
                            deposit: safe_deposit.0.into(),
                        },
                    ))
                }
            })
            .collect()
    }

    /// Create signature request from transaction and required parameters
    fn create_signature_request(
        &self,
        tx: &omni_transaction::near::NearTransaction,
        derivation_path: String,
        domain_id: Option<u32>,
    ) -> serde_json::Value {
        let hashed_payload = utils::hash_payload(&tx.build_for_signing());

        let sign_request = SignRequest {
            payload_v2: EddsaPayload {
                ecdsa: hex::encode(hashed_payload),
            },
            path: derivation_path,
            domain_id: domain_id.unwrap_or(NEAR_MPC_DOMAIN_ID),
        };

        serde_json::json!({ "request": sign_request })
    }

    /// Convert deposit numbers to strings in JSON
    fn convert_deposits_to_strings(&self, json_string: String) -> Result<String, String> {
        let mut result = json_string.clone();

        // This regex matches deposit values which are numbers
        use regex::Regex;
        let re = Regex::new(r#""deposit"\s*:\s*(\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)"#)
            .map_err(|e| format!("Failed to compile regex: {}", e))?;

        // Convert the matched deposit value groups into strings
        result = re
            .replace_all(&result, |caps: &regex::Captures| {
                let number_str = &caps[1];
                format!("\"deposit\":\"{}\"", number_str)
            })
            .to_string();

        Ok(result)
    }

    // Request a signature from the MPC signer
    #[payable]
    pub fn request_signature(
        &mut self,
        contract_id: AccountId,
        actions_json: String,
        nonce: U64,
        block_hash: Base58CryptoHash,
        mpc_signer_pk: String,
        derivation_path: String,
        domain_id: Option<u32>,
    ) -> Promise {
        let total_gas_start = env::prepaid_gas();
        near_sdk::env::log_str(&format!(
            "Starting request_signature with {} TGas",
            total_gas_start.as_tgas()
        ));

        let attached_gas = env::prepaid_gas();
        let required_gas =
            GAS_FOR_REQUEST_SIGNATURE.saturating_add(BASE_GAS.saturating_add(CALLBACK_GAS));

        if attached_gas < required_gas {
            env::panic_str(&format!(
                "Not enough gas attached. Please attach at least {} TGas. Attached: {} TGas",
                required_gas.as_tgas(),
                attached_gas.as_tgas()
            ));
        }

        if !self
            .authorized_users
            .contains(&env::predecessor_account_id())
        {
            env::panic_str("Unauthorized: only authorized users can request signatures");
        }

        // Parse actions from JSON string
        let actions: Vec<ActionString> = match serde_json::from_str(&actions_json) {
            Ok(actions) => actions,
            Err(e) => {
                env::panic_str(&format!("Failed to parse actions JSON: {}", e));
            }
        };

        // Validate and build OmniActions
        let omni_actions = match self.validate_and_build_actions(actions, &contract_id) {
            Ok(actions) => actions,
            Err(e) => {
                env::panic_str(&e);
            }
        };
        let gas_after_validation = attached_gas.saturating_sub(env::used_gas());

        near_sdk::env::log_str(&format!(
            "Gas remaining after validation: {} TGas",
            gas_after_validation.as_tgas()
        ));

        // construct the entire transaction to be signed
        let tx = TransactionBuilder::new::<NEAR>()
            .signer_id(env::current_account_id().to_string())
            .signer_public_key(match mpc_signer_pk.to_public_key() {
                Ok(pk) => pk,
                Err(e) => {
                    env::panic_str(&format!("Invalid public key format: {}", e));
                }
            })
            .nonce(nonce.0) // Use the provided nonce
            .receiver_id(contract_id.to_string())
            .block_hash(OmniBlockHash(block_hash.into()))
            .actions(omni_actions.clone())
            .build();

        // Log transaction details
        near_sdk::env::log_str(&format!(
            "Transaction details before signing:
            - Signer ID: {}
            - Receiver ID: {}
            - derivation_path: {}
            - Signer Public Key: {}
            - Number of Actions: {}",
            tx.signer_id,
            tx.receiver_id,
            derivation_path,
            mpc_signer_pk,
            tx.actions.len()
        ));

        // Serialize transaction into a string to pass into callback
        let tx_json_string = match serde_json::to_string(&tx) {
            Ok(s) => s,
            Err(e) => {
                env::panic_str(&format!("Failed to serialize NearTransaction: {:?}", e));
            }
        };

        // Convert large deposit numbers to strings for JSON compatibility
        let modified_tx_string = match self.convert_deposits_to_strings(tx_json_string) {
            Ok(s) => s,
            Err(e) => {
                env::panic_str(&format!("Failed to convert deposits to strings: {}", e));
            }
        };

        near_sdk::env::log_str(&format!("near tx in json: {}", modified_tx_string));

        near_sdk::env::log_str(&format!(
            "Transaction details - Receiver: {}, Signer: {}, Actions: {:?}, Nonce: {}, BlockHash: {:?}",
            contract_id,
            env::current_account_id(),
            omni_actions,
            nonce.0,
            block_hash
        ));

        // Create signature request
        let request_payload =
            self.create_signature_request(&tx, derivation_path.clone(), domain_id);

        let request_payload_bytes = match near_sdk::serde_json::to_vec(&request_payload) {
            Ok(bytes) => bytes,
            Err(e) => {
                env::panic_str(&format!("Failed to serialize request payload: {}", e));
            }
        };

        // Call MPC requesting a signature for the above txn
        let total_gas_remaining = total_gas_start.saturating_sub(env::used_gas());
        near_sdk::env::log_str(&format!(
            "Gas remaining for signature request: {} TGas",
            total_gas_remaining.as_tgas()
        ));
        Promise::new(self.signer_id.clone())
            .function_call(
                "sign".to_string(),
                request_payload_bytes,
                env::attached_deposit(),
                total_gas_remaining,
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(CALLBACK_GAS)
                    .sign_request_callback(modified_tx_string),
            )
    }

    pub fn add_full_access_key(&mut self, public_key: PublicKey) -> Promise {
        self.assert_owner();
        Promise::new(env::current_account_id()).add_full_access_key(public_key)
    }

    #[payable]
    pub fn add_full_access_key_and_register_with_intents(
        &mut self,
        public_key: PublicKey,
    ) -> Promise {
        assert_eq!(
            env::attached_deposit(),
            NearToken::from_yoctonear(1),
            "This method requires an attached deposit of exactly 1 yoctoNear"
        );
        let request_payload = serde_json::json!({ "public_key": public_key });
        let request_payload_bytes = match near_sdk::serde_json::to_vec(&request_payload) {
            Ok(bytes) => bytes,
            Err(e) => {
                near_sdk::env::log_str(&format!("Failed to serialize request payload: {}", e));
                // Return a promise that will fail, but don't panic. Avoiding panic blocks a DoS path that would otherwise burn gas.
                return Promise::new(env::current_account_id()).function_call(
                    "panic".to_string(),
                    format!("Failed to serialize request payload: {}", e).into_bytes(),
                    NearToken::from_yoctonear(0),
                    Gas::from_tgas(1),
                );
            }
        };

        self.add_full_access_key(public_key).then(
            Promise::new(NEAR_INTENTS_ADDRESS.clone()).function_call(
                "add_public_key".to_string(),
                request_payload_bytes,
                env::attached_deposit(),
                BASE_GAS,
            ),
        )
    }

    #[private] // Only callable by the contract itself
    pub fn sign_request_callback(
        &mut self,
        #[callback_result] call_result: Result<SignatureResponse, PromiseError>,
        tx_json_string: String,
    ) -> String {
        let response = match call_result {
            Ok(response) => {
                near_sdk::env::log_str(&format!("Parsed JSON response: {:?}", response));
                response
            }
            Err(e) => {
                near_sdk::env::log_str(&format!("Failed to parse JSON: {:?}", e));
                return "ERROR: Failed to parse response JSON".to_string();
            }
        };

        // Deserialize transaction
        let near_tx = match serde_json::from_str::<models::NearTransaction>(&tx_json_string) {
            Ok(tx) => tx,
            Err(e) => {
                near_sdk::env::log_str(&format!(
                    "Failed to deserialize transaction: {} - JSON: {}",
                    e, tx_json_string
                ));
                return "ERROR: Failed to deserialize transaction".to_string();
            }
        };

        let message_hash = utils::hash_payload(&near_tx.build_for_signing());

        // Handle different signature formats
        let omni_signature = match response {
            SignatureResponse::Eddsa(eddsa) => {
                near_sdk::env::log_str("Using ED25519 signature format");
                if eddsa.signature.len() < 64 {
                    near_sdk::env::log_str(&format!(
                        "Invalid ED25519 signature length: expected 64, got {}",
                        eddsa.signature.len()
                    ));
                    return "ERROR: Invalid ED25519 signature length".to_string();
                }
                let r = match eddsa.signature[0..32].try_into() {
                    Ok(r) => r,
                    Err(_) => {
                        near_sdk::env::log_str("Failed to convert ED25519 r component to array");
                        return "ERROR: Failed to convert ED25519 r component".to_string();
                    }
                };
                let s = match eddsa.signature[32..64].try_into() {
                    Ok(s) => s,
                    Err(_) => {
                        near_sdk::env::log_str("Failed to convert ED25519 s component to array");
                        return "ERROR: Failed to convert ED25519 s component".to_string();
                    }
                };
                Signature::ED25519(ED25519Signature { r, s })
            }
            SignatureResponse::Ecdsa(ecdsa) => {
                near_sdk::env::log_str("Using SECP256K1 signature format");
                // Convert signature components
                let r = match hex::decode(&ecdsa.big_r.affine_point[2..]) {
                    Ok(r) => r,
                    Err(e) => {
                        near_sdk::env::log_str(&format!("Invalid hex in r: {}", e));
                        return "ERROR: Invalid hex in r".to_string();
                    }
                };
                let s = match hex::decode(&ecdsa.s.scalar) {
                    Ok(s) => s,
                    Err(e) => {
                        near_sdk::env::log_str(&format!("Invalid hex in s: {}", e));
                        return "ERROR: Invalid hex in s".to_string();
                    }
                };
                let v = ecdsa.recovery_id;

                // Combine r and s for verification
                let mut signature = Vec::with_capacity(64);
                signature.extend_from_slice(&r);
                signature.extend_from_slice(&s);

                // Verify signature
                let recovered = self.test_recover(message_hash.to_vec(), signature, v);
                match recovered {
                    Some(public_key) => {
                        near_sdk::env::log_str(&format!(
                            "Signature verified! Recovered public key: {}",
                            public_key
                        ));
                    }
                    None => {
                        near_sdk::env::log_str("Signature verification failed!");
                        return "ERROR: Invalid signature: ecrecover failed".to_string();
                    }
                }

                // Add individual bytes together in the correct order
                let mut signature_bytes = [0u8; 65];
                signature_bytes[..32].copy_from_slice(&r);
                signature_bytes[32..64].copy_from_slice(&s);
                signature_bytes[64] = v;

                // Create signature
                Signature::SECP256K1(Secp256K1Signature(signature_bytes))
            }
        };

        // Add signature to transaction
        let near_tx_signed = near_tx.build_with_signature(omni_signature);

        let base64_tx =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &near_tx_signed);
        near_sdk::env::log_str(&format!("Signed transaction (base64): {}", base64_tx));

        base64_tx
    }

    fn test_recover(&self, hash: Vec<u8>, signature: Vec<u8>, v: u8) -> Option<String> {
        let recovered: Option<[u8; 64]> = env::ecrecover(&hash, &signature, v, true);

        recovered.map(|key: [u8; 64]| {
            // Add prefix byte for secp256k1 (0x01)
            let mut prefixed_key = vec![0x01];
            prefixed_key.extend_from_slice(&key);

            let key = format!("secp256k1:{}", bs58::encode(&prefixed_key).into_string());

            key
        })
    }
}
