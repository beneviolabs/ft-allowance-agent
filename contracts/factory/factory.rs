use bs58;
use near_sdk::serde::Serialize;
use near_sdk::{
    AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseError, PublicKey, env, near,
};

const NEAR_PER_STORAGE: NearToken = NearToken::from_yoctonear(10u128.pow(19));
const PROXY_CODE: &[u8] = include_bytes!("../target/near/proxy_contract.wasm");
const TESTNET_SIGNER: &str = "v1.signer-prod.testnet";
const MAINNET_SIGNER: &str = "v1.signer";

mod unit_tests;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct ProxyFactory {
    signer_contract: AccountId,
    global_proxy_base58_hash: Vec<u8>,
    owner_id: AccountId,
}

#[near]
impl ProxyFactory {
    #[init]
    pub fn new(network: String, global_proxy_base58_hash: String) -> Self {
        assert!(!env::state_exists(), "Already initialized");

        let signer_contract = match network.as_str() {
            "mainnet" => MAINNET_SIGNER,
            _ => TESTNET_SIGNER,
        }
        .parse()
        .unwrap();

        Self {
            signer_contract,
            global_proxy_base58_hash: Self::decode_code_hash(&global_proxy_base58_hash),
            owner_id: env::current_account_id(),
        }
    }

    #[payable]
    pub fn deposit_and_create_proxy_global(&mut self, owner_id: AccountId) -> Promise {
        let deposit = env::attached_deposit();
        assert!(
            deposit >= NearToken::from_yoctonear(1_000_000),
            "Must attach at least 1000 yⓃ" // TODO: make this amount more precise - how much of a deposit must one add? 0.0025?
        );

        self.create_proxy_global(owner_id.clone()).then(
            Self::ext(env::current_account_id())
                .on_proxy_created(env::predecessor_account_id(), deposit),
        )
    }

    #[payable]
    pub fn create_proxy_global(&mut self, owner_id: AccountId) -> Promise {
        let trimmed_owner = self.get_base_account_name(&owner_id);
        let full_sub_account: AccountId =
            format!("{}.{}", trimmed_owner, env::current_account_id())
                .parse()
                .unwrap();

        env::log_str(&format!(
            "Creating proxy with global contract - Account: {}, Owner: {}, Signer: {}, bs58 Code Hash: {}",
            full_sub_account,
            owner_id,
            self.signer_contract,
            bs58::encode(&self.global_proxy_base58_hash).into_string()
        ));

        Promise::new(full_sub_account.clone())
            .create_account()
            .transfer(env::attached_deposit())
            .use_global_contract(self.global_proxy_base58_hash.clone())
            .function_call(
                "new".to_string(),
                near_sdk::serde_json::to_vec(&ProxyInitArgs {
                    owner_id,
                    signer_id: self.signer_contract.clone(),
                })
                .unwrap(),
                NearToken::from_near(0),
                Gas::from_tgas(50),
            )
    }

    #[payable]
    pub fn deposit_and_create_proxy(&mut self, owner_id: AccountId) -> Promise {
        let deposit = env::attached_deposit();
        assert!(
            //TODO update this to use NEP-591 to reduce the required near. https://github.com/near/NEPs/pull/591/files
            deposit >= NearToken::from_yoctonear(3_800_000_000_000_000_000_000_000),
            "Must attach at least 3.8 NEAR"
        );

        self.create_proxy(owner_id.clone()).then(
            Self::ext(env::current_account_id())
                .on_proxy_created(env::predecessor_account_id(), deposit),
        )
    }

    #[payable]
    pub fn create_proxy(&mut self, owner_id: AccountId) -> Promise {
        assert!(
            env::attached_deposit() >= NEAR_PER_STORAGE,
            "Attach at least 1 yⓃ"
        );
        let trimmed_owner = self.get_base_account_name(&owner_id);
        let full_sub_account: AccountId =
            format!("{}.{}", trimmed_owner, env::current_account_id())
                .parse()
                .unwrap();

        // Verify deployment conditions
        let attached = env::attached_deposit();
        let code = PROXY_CODE;
        let contract_bytes = code.len() as u128;
        let contract_storage_cost = NEAR_PER_STORAGE.saturating_mul(contract_bytes);
        let minimum_needed = contract_storage_cost.saturating_add(NearToken::from_millinear(100));

        assert!(
            attached >= minimum_needed,
            "Attach at least {minimum_needed} yⓃ"
        );

        // This is one transaction with multiple action receipts. The entire transaction will be "rolled-back" if an action fails, like https://testnet.nearblocks.io/txns/2oQzUR7RV4v69t7VZaLP8AiZBrh3rdULVMQx8bury9A6
        Promise::new(full_sub_account.clone())
            .create_account()
            .transfer(env::attached_deposit())
            .deploy_contract(code.to_vec())
            .function_call(
                "new".to_string(),
                near_sdk::serde_json::to_vec(&ProxyInitArgs {
                    owner_id,
                    signer_id: self.signer_contract.clone(),
                })
                .unwrap(),
                NearToken::from_near(0),
                Gas::from_tgas(50),
            )
    }

    #[private]
    pub fn on_proxy_created(
        &mut self,
        original_caller: AccountId,
        #[callback_result] creation_result: Result<(), PromiseError>,
        deposit: NearToken,
    ) -> Promise {
        if creation_result.is_err() {
            env::log_str("Proxy creation failed, refunding deposit");
            Promise::new(original_caller).transfer(deposit)
        } else {
            env::log_str("Proxy created successfully");
            Promise::new(env::current_account_id())
        }
    }

    fn get_base_account_name(&self, owner_id: &AccountId) -> String {
        let base_address =
            if owner_id.as_str().ends_with(".testnet") || owner_id.as_str().ends_with(".near") {
                // Take everything before .testnet or .near
                let parts: Vec<&str> = owner_id.as_str().rsplitn(2, '.').collect();
                parts[1].to_string()
            } else {
                owner_id.to_string()
            };

        base_address
    }

    /// Helper function to decode Base58 code hash string to 32-byte hash
    fn decode_code_hash(code_hash_str: &str) -> Vec<u8> {
        let decoded_hash = bs58::decode(code_hash_str)
            .into_vec()
            .unwrap_or_else(|e| panic!("Failed to decode Base58 code hash: {}", e));

        // Verify it's exactly 32 bytes (SHA-256 hash)
        assert_eq!(decoded_hash.len(), 32, "Code hash must be exactly 32 bytes");

        decoded_hash
    }

    /// Set global code hash from Base58 string
    pub fn set_global_code_hash(&mut self, code_hash_str: String) {
        self.assert_owner();

        self.global_proxy_base58_hash = Self::decode_code_hash(&code_hash_str);

        env::log_str(&format!(
            "Global proxy code hash updated to: {}",
            code_hash_str
        ));
    }

    pub fn add_full_access_key(&mut self, public_key: PublicKey) -> Promise {
        self.assert_owner();
        Promise::new(env::current_account_id()).add_full_access_key(public_key)
    }

    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "Only the contract owner can perform this action."
        );
    }

    // View methods
    pub fn get_proxy_code_base58_hash(&self) -> String {
        bs58::encode(&self.global_proxy_base58_hash).into_string()
    }

    pub fn get_proxy_code_hash_hex(&self) -> String {
        hex::encode(&self.global_proxy_base58_hash)
    }

    pub fn get_signer_contract(&self) -> AccountId {
        self.signer_contract.clone()
    }
}

#[cfg(test)]
impl ProxyFactory {
    pub(crate) fn test_get_base_account_name(&self, owner_id: &AccountId) -> String {
        self.get_base_account_name(owner_id)
    }
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
struct ProxyInitArgs {
    owner_id: AccountId,
    signer_id: AccountId,
}
