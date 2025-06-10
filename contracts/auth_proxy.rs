use actions::NearAction;
use hex::FromHex;
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
        FunctionCallAction as OmniFunctionCallAction, Signature, U64 as OmniU64, U128 as OmniU128,
    },
};

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
            .signer_public_key(mpc_signer_pk.to_public_key().unwrap())
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
                &format!("\"deposit\":{}", deposit.0),
                &format!("\"deposit\":\"{}\"", deposit.0),
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

        // SHA-256 hash of the serialized transaction
        let hashed_payload = utils::hash_payload(&tx.build_for_signing());

        // Create a signature request for the hashed payload
        let request = SignRequest {
            payload_v2: EddsaPayload {
                ecdsa: hex::encode(hashed_payload),
            },
            path: derivation_path,
            domain_id: 0, //TODO make this a param so clients can request siggys for addresses on different chains
        };

        let request_payload = serde_json::json!({ "request": request });

        // Call MPC requesting a signature for the above txn
        Promise::new(self.signer_id.clone())
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

    pub fn add_full_access_key(&mut self, public_key: PublicKey) {
        self.assert_owner();
        Promise::new(env::current_account_id()).add_full_access_key(public_key);
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
                panic!("Failed to parse response JSON");
            }
        };

        // Deserialize transaction
        let near_tx = serde_json::from_str::<models::NearTransaction>(&tx_json_string)
            .unwrap_or_else(|_| panic!("Failed to deserialize transaction: {:?}", tx_json_string));

        // Handle different signature formats
        let omni_signature = match response {
            SignatureResponse::Eddsa(eddsa) => {
                near_sdk::env::log_str("Using ED25519 signature format");
                Signature::ED25519(ED25519Signature {
                    r: eddsa.signature[0..32].try_into().unwrap(),
                    s: eddsa.signature[32..64].try_into().unwrap(),
                })
            }
            SignatureResponse::Ecdsa(ecdsa) => {
                near_sdk::env::log_str("Using SECP256K1 signature format");
                // Big R value from the MPC signature
                let big_r = ecdsa.big_r.affine_point;
                let scalar = ecdsa.s.scalar;
                let recovery_id = ecdsa.recovery_id;
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
                Signature::SECP256K1(Secp256K1Signature(signature_bytes))
            }
        };

        near_sdk::env::log_str(&format!(
            "constructed omni signature: {:?}",
            &omni_signature
        ));

        // Add signature to transaction
        let near_tx_signed = near_tx.build_with_signature(omni_signature);

        let base64_tx =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &near_tx_signed);
        near_sdk::env::log_str(&format!("Signed transaction (base64): {}", base64_tx));

        base64_tx
    }
}
