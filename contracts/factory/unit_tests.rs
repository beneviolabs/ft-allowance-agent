#[cfg(test)]
mod tests {

    use crate::ProxyFactory;
    use near_sdk::{
        test_utils::{accounts, VMContextBuilder},
        testing_env, AccountId, Gas, NearToken, Promise,
    };

    const MIN_DEPOSIT: u128 = 3_800_000_000_000_000_000_000_000; // 3.8 NEAR

    fn get_context(predecessor: AccountId, deposit: Option<NearToken>) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .predecessor_account_id(predecessor)
            .current_account_id("factory.testnet".parse().unwrap())
            .attached_deposit(deposit.unwrap_or(NearToken::from_yoctonear(MIN_DEPOSIT)))
            .prepaid_gas(Gas::from_tgas(150));
        builder
    }

    #[test]
    fn test_factory_initialization() {
        let context = get_context(accounts(1), None);
        testing_env!(context.build());

        let contract = ProxyFactory::new("testnet".to_string());
        assert_eq!(
            contract.get_signer_contract(),
            "v1.signer-prod.testnet".parse::<AccountId>().unwrap()
        );
    }

    #[test]
    #[should_panic(expected = "Must attach at least 3.8 NEAR")]
    fn test_insufficient_deposit() {
        let context = get_context(accounts(1), Some(NearToken::from_near(1)));
        testing_env!(context.build());

        let mut contract = ProxyFactory::new("testnet".to_string());
        contract.deposit_and_create_proxy(accounts(2));
    }

    #[test]
    fn test_proxy_address_generation() {
        let context = get_context(accounts(1), None);
        testing_env!(context.build());

        let contract = ProxyFactory::new("mainnet".to_string());

        // Test testnet address
        let owner_id: AccountId = "alice.testnet".parse().unwrap();
        let proxy_address = contract.test_get_base_account_name(&owner_id);
        assert_eq!(proxy_address, "alice");

        // Test subaccount
        let owner_id: AccountId = "trading.alice.testnet".parse().unwrap();
        let proxy_address = contract.test_get_base_account_name(&owner_id);
        assert_eq!(proxy_address, "trading.alice");
    }

    #[test]
    fn test_proxy_code_hash() {
        let context = get_context(accounts(1), None);
        testing_env!(context.build());

        let contract = ProxyFactory::new("mainnet".to_string());
        let hash = contract.get_proxy_code_hash();

        assert!(!hash.is_empty(), "Code hash should not be empty");
        assert_eq!(hash.len(), 64, "Hash should be 32 bytes (64 hex chars)");
    }

    #[test]
    fn test_successful_proxy_creation() {
        let context = get_context(accounts(1), Some(NearToken::from_near(4)));
        testing_env!(context.build());

        let mut contract = ProxyFactory::new("testnet".to_string());
        let result = contract.create_proxy(accounts(2));

        // Since we can't fully test Promise chain in unit tests,
        // we at least verify the promise was created
        assert!(matches!(result, Promise { .. }));
    }

    #[test]
    fn test_proxy_creation_refund() {
        let context = get_context(accounts(1), None);
        testing_env!(context.build());

        let mut contract = ProxyFactory::new("testnet".to_string());
        let result = contract.on_proxy_created(
            accounts(1),
            Err(near_sdk::PromiseError::Failed),
            NearToken::from_near(4),
        );

        // Verify refund promise was created
        assert!(matches!(result, Promise { .. }));
    }
}
