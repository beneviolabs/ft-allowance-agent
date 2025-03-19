use std::str::FromStr;

use near_gas::NearGas;
use near_sdk::collections::UnorderedSet;
use near_sdk::ext_contract;
use near_sdk::json_types::{Base58CryptoHash, U64};
use near_sdk::{
    bs58, env, near, AccountId, CurveType, Gas, NearToken, PanicOnDefault, Promise, PromiseError,
    PublicKey,
};
use near_sdk::base64;
use hex::FromHex;
use omni_transaction::near::types::Secp256K1PublicKey;
use omni_transaction::transaction_builder::TransactionBuilder;
use omni_transaction::transaction_builder::TxBuilder;
use omni_transaction::near::near_transaction::NearTransaction;
use omni_transaction::{
    near::types::{
        Action as OmniAction, BlockHash as OmniBlockHash,
        FunctionCallAction as OmniFunctionCallAction, PublicKey as OmniPublicKey, U128 as OmniU128, Secp256K1Signature, Signature,
        U64 as OmniU64,
    },
    NEAR,
};
use sha2::{Digest, Sha256};

#[ext_contract(ext_self)]
pub trait ExtSelf {
    fn callback_method(&mut self, #[callback_result] call_result: Result<Vec<u8>, PromiseError>);
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BigR {
    pub affine_point: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScalarValue {
    pub scalar: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SignatureResponse {
    pub big_r: BigR,
    pub s: ScalarValue,
    pub recovery_id: u8,
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

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct SignRequest {
    pub payload: Vec<u8>,
    pub path: String,
    pub key_version: u32,
}

#[derive(Clone)]
#[near(serializers = [json, borsh])]
pub struct NearAction {
    pub method_name: String,
    pub contract_id: AccountId,
    pub gas_attached: Gas,
    pub deposit_attached: NearToken,
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
        self.assert_action_allowed(&action);

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
            .signer_public_key(Self::convert_pk_to_omni(&PublicKey::from_str(&mpc_signer_pk).unwrap()))
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
        let hashed_payload = ProxyContract::hash_payload(&tx.build_for_signing());

        // Log arguments used for signature request
        near_sdk::env::log_str(&format!(
            "Signing request - transaction hash: {:?}, Path: {}, Key Version: {}",
            bs58::encode(&hashed_payload).into_string(),
            "ed25519:71BRk6DLCgBRrQe6KLyPQGNiXoPJGKhP79Mjyeucu2fN",
            0
        ));
        // Create a signature request for the hashed payload
        let request = SignRequest {
            payload: hashed_payload.to_vec(),
            path: "ed25519:71BRk6DLCgBRrQe6KLyPQGNiXoPJGKhP79Mjyeucu2fN".to_string(), // TODO add this pk value programmatically
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

    fn convert_pk_to_omni(pk: &PublicKey) -> omni_transaction::near::types::PublicKey {
        // TODO We might need to expand this to support ETH/other curve types
        let public_key_data = &pk.as_bytes()[1..]; // Skipping the first byte which is the curve type
        //const ED25519_PUBLIC_KEY_LENGTH: usize = 32;
        const SECP256K1_PUBLIC_KEY_LENGTH: usize = 64;
        let ed25519_key: [u8; SECP256K1_PUBLIC_KEY_LENGTH] = public_key_data
            .try_into()
            .expect("Failed to convert public key");

        OmniPublicKey::SECP256K1(Secp256K1PublicKey::from(ed25519_key))
    }

    fn hash_payload(payload: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(payload);
        hasher.finalize().into()
    }

    /// Converts a `PublicKey` to a string representation.
    fn public_key_to_string(public_key: &PublicKey) -> String {
        let curve_type = public_key.curve_type();
        let encoded = bs58::encode(&public_key.as_bytes()[1..]).into_string(); // Skipping the first byte which is the curve type
        match curve_type {
            CurveType::ED25519 => format!("ed25519:{}", encoded),
            CurveType::SECP256K1 => format!("secp256k1:{}", encoded),
        }
    }

    fn assert_action_allowed(&self, action: &NearAction) {
        let allowed_contracts = ["wrap.near", "intents.near", "wrap.testnet"];
        let restricted_methods = ["deposit", "add_public_key"];
        if !allowed_contracts.contains(&action.contract_id.as_str()) {
            panic!(
                "{} is not allowed. Only wrap.near, wrap.testnet, and intents.near are permitted",
                action.contract_id
            );
        }
        if restricted_methods.contains(&action.method_name.as_str()) {
            panic!("Method {} is restricted", action.method_name);
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

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::{
        test_utils::{accounts, VMContextBuilder},
        testing_env,
    };

    fn get_context(predecessor: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder.predecessor_account_id(predecessor);
        builder
    }

    #[test]
    fn test_new() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = ProxyContract::new(accounts(1));
        assert_eq!(contract.owner_id, accounts(1));
    }

    #[test]
    fn test_authorize_user() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = ProxyContract::new(accounts(1));

        contract.add_authorized_user(accounts(2));
        assert!(contract.is_authorized(accounts(2)));
    }

    #[test]
    fn test_remove_authorized_user() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = ProxyContract::new(accounts(1));

        contract.add_authorized_user(accounts(2));
        assert!(contract.is_authorized(accounts(2)));

        contract.remove_authorized_user(accounts(2));
        assert!(!contract.is_authorized(accounts(2)));
    }

    #[test]
    #[should_panic(expected = "Be gone. You have no power here.")]
    fn test_unauthorized_add_user() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = ProxyContract::new(accounts(1));
        contract.add_authorized_user(accounts(3));
    }

    #[test]
    fn test_get_authorized_users() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = ProxyContract::new(accounts(1));

        contract.add_authorized_user(accounts(2));
        contract.add_authorized_user(accounts(3));

        let users = contract.get_authorized_users();
        assert_eq!(users.len(), 2);
        assert!(users.contains(&accounts(2)));
        assert!(users.contains(&accounts(3)));
    }

    #[test]
    fn test_set_signer_contract() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = ProxyContract::new(accounts(1));

        contract.set_signer_contract(accounts(2));
        assert_eq!(contract.get_signer_contract(), accounts(2));
    }

    #[test]
    #[should_panic(expected = "Be gone. You have no power here.")]
    fn test_unauthorized_set_signer() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = ProxyContract::new(accounts(1));
        contract.set_signer_contract(accounts(3));
    }

    #[test]
    #[should_panic(expected = "Unauthorized: only authorized users can request signatures")]
    fn test_unauthorized_request_signature() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = ProxyContract::new(accounts(1));
        contract.request_signature(
            accounts(3),                       // contract_id: AccountId
            "test_method".to_string(),         // method_name: String
            vec![1, 2, 3],                     // args: Vec<u8>
            near_sdk::json_types::U64(10),                // gas: U64
            NearToken::from_near(1),           // deposit: NearToken
            U64(1),                            // nonce: U64
            Base58CryptoHash::from([0u8; 32]), // block_hash: Base58CryptoHash
            "secp256k1:abcd1234".to_string(),
        );
    }

    #[test]
    #[should_panic(
        expected = "danny is not allowed. Only wrap.near, wrap.testnet, and intents.near are permitted"
    )]
    fn test_disallowed_action() {
        //TODO rewrite this as a workspace integration test
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = ProxyContract::new(accounts(1));

        testing_env!(get_context(accounts(1)).build());
        contract.add_authorized_user(accounts(2));

        testing_env!(get_context(accounts(2)).build());
        contract.request_signature(
            accounts(3),                       // contract_id
            "test_method".to_string(),         // method_name
            vec![1, 2, 3],                     // args
            near_sdk::json_types::U64(10),                // gas
            NearToken::from_near(1),           // deposit
            U64(1),                            // nonce
            Base58CryptoHash::from([0u8; 32]), // block_hash
            "secp256k1:abcd1234".to_string(),
        );
    }

    #[test]
    fn test_successful_request_signature() {
        let context = get_context(accounts(1));
        testing_env!(context.build());

        let mut contract = ProxyContract::new(accounts(1));
        contract.add_authorized_user(accounts(2));

        testing_env!(get_context(accounts(2)).build());

        contract.request_signature(
            "wrap.near".parse().unwrap(),
            "ft_transfer".to_string(),
            vec![1, 2, 3],
            near_sdk::json_types::U64(10),
            NearToken::from_near(1),
            U64(1),
            Base58CryptoHash::from([0u8; 32]),
            "secp256k1:abcd1234".to_string(),
        );
    }
}
