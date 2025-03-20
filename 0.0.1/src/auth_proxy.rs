use std::str::FromStr;

use actions::NearAction;
use near_gas::NearGas;
use near_sdk::collections::UnorderedSet;
use near_sdk::ext_contract;
use near_sdk::json_types::{Base58CryptoHash, U64};
use near_sdk::{
    bs58, env, near, AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseError,
    PublicKey,
};
use near_sdk::base64;
use hex::FromHex;
use omni_transaction::transaction_builder::TransactionBuilder;
use omni_transaction::transaction_builder::TxBuilder;
use omni_transaction::near::near_transaction::NearTransaction;
use omni_transaction::{
    near::types::{
        Action as OmniAction, BlockHash as OmniBlockHash,
        FunctionCallAction as OmniFunctionCallAction, U128 as OmniU128, Secp256K1Signature, Signature,
        U64 as OmniU64,
    },
    NEAR,
};

mod models;
mod actions;
mod utils;

pub use crate::models::*;

#[ext_contract(ext_self)]
pub trait ExtSelf {
    fn callback_method(&mut self, #[callback_result] call_result: Result<Vec<u8>, PromiseError>);
}


const GAS_FOR_REQUEST_SIGNATURE: Gas = Gas::from_tgas(50);
const BASE_GAS: Gas = Gas::from_tgas(5);  // Base gas for contract execution
const CALLBACK_GAS: Gas = Gas::from_tgas(10); // Gas reserved for callback

const TESTNET_SIGNER: &str = "v1.signer-prod.testnet";
//const MAINNET_SIGNER: &str = "v1.signer";

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
        Self {
            owner_id,
            authorized_users: UnorderedSet::new(b"a"),
            signer_contract: TESTNET_SIGNER.parse().unwrap(),
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
    pub fn request_signature(
        &mut self,
        contract_id: AccountId,
        method_name: String,
        args: Vec<u8>,
        gas: U64,
        deposit: NearToken,
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

        near_sdk::env::log_str(&format!(
            "Request received - Contract: {}, Method: {}, Gas: {}, Deposit: {}, Nonce: {}, Block Hash: {:?}",
            contract_id,
            method_name,
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

        let actions = vec![OmniAction::FunctionCall(Box::new(OmniFunctionCallAction {
            method_name: method_name.clone(),
            args: args.clone(),
            gas: OmniU64(gas.into()),
            deposit: OmniU128(deposit.as_yoctonear()),
        }))];

        near_sdk::env::log_str(&format!("Near Token deposit amount: {}", deposit));

        // construct the entire transaction to be signed
        let tx = TransactionBuilder::new::<NEAR>()
            .signer_id(env::current_account_id().to_string())
            .signer_public_key(utils::convert_pk_to_omni(&PublicKey::from_str(&mpc_signer_pk).unwrap()))
            .nonce(nonce.0) // Use the provided nonce
            .receiver_id(contract_id.to_string())
            .block_hash(OmniBlockHash(block_hash.into()))
            .actions(actions.clone())
            .build();

        // Serialize transaction into a string to pass into callback
        let tx_json_string = serde_json::to_string(&tx)
            .unwrap_or_else(|e| panic!("Failed to serialize NearTransaction: {:?}", e)).replace("1000000000000000000000000", "\"1000000000000000000000000\""); // TODO Temp fix

        near_sdk::env::log_str(&format!("near tx in json: {}", tx_json_string));

        near_sdk::env::log_str(&format!(
            "Transaction details - Receiver: {}, Signer: {}, Actions: {:?}, Nonce: {}, BlockHash: {:?}",
            contract_id,
            self.owner_id,
            actions,
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
    ) -> Vec<u8>{
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

        // Deserialize transaction
        let near_tx = serde_json::from_str::<NearTransaction>(&tx_json_string)
                .unwrap_or_else(|_| panic!("Failed to deserialize transaction: {:?}", tx_json_string));

        // Add signature to transaction
        let near_tx_signed = near_tx.build_with_signature(omni_signature);

        let base64_tx = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &near_tx_signed);
        near_sdk::env::log_str(&format!("Signed transaction (base64): {}", base64_tx));

        near_tx_signed

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
