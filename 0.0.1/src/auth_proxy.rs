use near_sdk::json_types::{Base58CryptoHash, U64};
use near_sdk::{bs58, env, near, near_bindgen, AccountId, CurveType, Gas, NearToken, PanicOnDefault, Promise, PublicKey};
use near_sdk::collections::UnorderedSet;
use near_gas::NearGas;
use sha2::{Sha256, Digest};
use omni_transaction::transaction_builder::TransactionBuilder;
use omni_transaction::transaction_builder::TxBuilder;
use omni_transaction::{
    near::types::{
        Action as OmniAction, BlockHash as OmniBlockHash,
        FunctionCallAction as OmniFunctionCallAction, U128 as OmniU128, U64 as OmniU64,
        ED25519PublicKey as OmniEd25519PublicKey, PublicKey as OmniPublicKey
    },
    NEAR,
};

const GAS_FOR_REQUEST_SIGNATURE: NearGas = NearGas::from_tgas(10);
const TESTNET_SIGNER: &str = "v1.signer-prod.testnet";
//const MAINNET_SIGNER: &str = "v1.signer";

#[near_bindgen]
#[derive(PanicOnDefault)]
pub struct ProxyContract {
    owner_id: AccountId,
    authorized_users: UnorderedSet<AccountId>,
    signer_contract: AccountId,
}

use borsh::{BorshSerialize, BorshDeserialize};

#[derive(BorshSerialize, BorshDeserialize)]
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
    pub fn request_signature(&mut self, contract_id: AccountId,
        method_name: String,
        args: Vec<u8>,
        gas: Gas,
        deposit: NearToken,
        nonce: U64,
        block_hash: Base58CryptoHash,) -> Promise {
        assert!(
            env::prepaid_gas() >= GAS_FOR_REQUEST_SIGNATURE,
            "Not enough gas attached. Please attach 10 TGas"
        );
        assert!(
            self.authorized_users.contains(&env::predecessor_account_id()),
            "Unauthorized: only authorized users can request signatures"
        );

        let action = NearAction {
            method_name: method_name.clone(),
            contract_id: contract_id.clone(),
            gas_attached: gas,
            deposit_attached: deposit,
        };

        // verify the action is permitted
        self.assert_action_allowed(&action);

        let actions = vec![OmniAction::FunctionCall(Box::new(OmniFunctionCallAction {
            method_name: method_name.clone(),
            args: args.clone(),
            gas: OmniU64(gas.as_gas()),
            deposit: OmniU128(deposit.as_yoctonear()),
        }))];

        // construct the entire transaction to be signed
        let tx = TransactionBuilder::new::<NEAR>()
            .signer_id(self.owner_id.to_string())
            .signer_public_key(ProxyContract::convert_pk_to_omni(&env::signer_account_pk())) //TODO perhaps this should be this contract's public key?
            .nonce(nonce.0) // Use the provided nonce
            .receiver_id(contract_id.to_string())
            .block_hash(OmniBlockHash(block_hash.into()))
            .actions(actions.clone())
            .build()
            .build_for_signing();

        // SHA-256 hash of the serialized transaction
        let hashed_payload = ProxyContract::hash_payload(&tx);

        // Create a signature request request for the hashed payload
        let request = SignRequest {
            payload: hashed_payload.to_vec(),
            path: ProxyContract::public_key_to_string(&env::signer_account_pk()),
            key_version: 0,
        };

        let mut request_bytes = Vec::new();
        request.serialize(&mut request_bytes).unwrap();

        // Call MPC requesting a signature for the above txn
        Promise::new(self.signer_contract.clone()).function_call(
            "sign".to_string(),
            request_bytes,
            env::attached_deposit(),
            GAS_FOR_REQUEST_SIGNATURE
        ).as_return()

    }

    fn convert_pk_to_omni(pk: &PublicKey) -> omni_transaction::near::types::PublicKey { // Might need to expand this to support ETH/other curve types
        let public_key_data = &pk.as_bytes()[1..]; // Skipping the first byte which is the curve type
        const ED25519_PUBLIC_KEY_LENGTH: usize = 32;
        let ed25519_key: [u8; ED25519_PUBLIC_KEY_LENGTH] = public_key_data
                .try_into()
                .expect("Failed to convert ED25519 public key");

        OmniPublicKey::ED25519(OmniEd25519PublicKey::from(ed25519_key))
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
          let allowed_contracts = ["wrap.near", "intents.near"];
          let restricted_methods = ["deposit", "add_public_key"];
          if !allowed_contracts.contains(&action.contract_id.as_str()) {
              panic!("Contract {} is not allowed. Only wrap.near and intents.near are permitted", action.contract_id);
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
        self.authorized_users.contains(&account_id)
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
    use near_sdk::{test_utils::{accounts, VMContextBuilder}, testing_env};

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
            accounts(3), // contract_id: AccountId
            "test_method".to_string(), // method_name: String
            vec![1, 2, 3], // args: Vec<u8>
            Gas::from_tgas(10), // gas: Gas
            NearToken::from_near(1), // deposit: NearToken
            U64(1), // nonce: U64
            Base58CryptoHash::from([0u8; 32]) // block_hash: Base58CryptoHash
        );
    }

    #[test]
    #[should_panic(expected = "danny is not allowed. Only wrap.near and intents.near are permitted")]
    fn test_disallowed_action() { //TODO rewrite this as a workspace integration test
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = ProxyContract::new(accounts(1));

        testing_env!(get_context(accounts(1)).build());
        contract.add_authorized_user(accounts(2));

        testing_env!(get_context(accounts(2)).build());
        contract.request_signature(
            accounts(3), // contract_id
            "test_method".to_string(), // method_name
            vec![1, 2, 3], // args
            Gas::from_tgas(10), // gas
            NearToken::from_near(1), // deposit
            U64(1), // nonce
            Base58CryptoHash::from([0u8; 32]) // block_hash
        );
    }

    #[test]
    fn test_successful_request_signature() {
        let context = get_context(accounts(1));
        testing_env!(context.build());

        let mut contract = ProxyContract::new(accounts(1));
        contract.add_authorized_user(accounts(2));

        testing_env!(get_context(accounts(2))
            .build()
        );

        contract.request_signature(
            "wrap.near".parse().unwrap(),
            "ft_transfer".to_string(),
            vec![1, 2, 3],
            Gas::from_tgas(10),
            NearToken::from_near(1),
            U64(1),
            Base58CryptoHash::from([0u8; 32])
        );

    }

}
