use near_sdk::serde::Serialize;
use near_sdk::{AccountId, Gas, NearToken, PanicOnDefault, Promise, env, near};

const NEAR_PER_STORAGE: NearToken = NearToken::from_yoctonear(10u128.pow(19));
const PROXY_CODE: &[u8] = include_bytes!("../target/near/proxy_contract.wasm");

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct ProxyFactory {
    owner_id: AccountId,
}

#[near]
impl ProxyFactory {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            owner_id: env::predecessor_account_id(), // TODO perhaps this can be removed completely since sub accounts can only be created by the factory owner
        }
    }

    //TODO update - this should return the deposit if the txn fails
    #[payable]
    pub fn deposit_and_create_proxy(&mut self, owner_id: AccountId) -> Promise {
        let deposit = env::attached_deposit();
        assert!(
            //TODO update this to use NEP-591 to reduce the required near. https://github.com/near/NEPs/pull/591/files
            deposit >= NearToken::from_yoctonear(3_800_000_000_000_000_000_000_000),
            "Must attach at least 3.8 NEAR"
        );
        self.create_proxy(owner_id)
    }

    #[payable]
    pub fn create_proxy(&mut self, owner_id: AccountId) -> Promise {
        assert!(
            env::attached_deposit() >= NEAR_PER_STORAGE,
            "Attach at least 1 yⓃ"
        );
        let trimmed_owner = owner_id.as_str().split('.').next().unwrap();
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
                near_sdk::serde_json::to_vec(&ProxyInitArgs { owner_id }).unwrap(),
                NearToken::from_near(0),
                Gas::from_tgas(50),
            )
    }

    // View methods
    pub fn get_proxy_code_hash(&self) -> String {
        hex::encode(env::sha256(PROXY_CODE))
    }
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
struct ProxyInitArgs {
    owner_id: AccountId,
}
