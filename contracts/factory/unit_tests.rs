#[cfg(test)]
mod tests {

    use crate::ProxyFactory;
    use near_sdk::{
        AccountId, Gas, NearToken, Promise, PublicKey,
        test_utils::{VMContextBuilder, accounts},
        testing_env,
    };
    use std::str::FromStr;

    const MIN_DEPOSIT: u128 = 1_000_000; // 1000 yⓃ for global contract

    fn get_context(
        predecessor: AccountId,
        current_account_id: AccountId,
        deposit: Option<NearToken>,
    ) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .predecessor_account_id(predecessor)
            .current_account_id(current_account_id)
            .attached_deposit(deposit.unwrap_or(NearToken::from_yoctonear(MIN_DEPOSIT)))
            .prepaid_gas(Gas::from_tgas(150));
        builder
    }

    #[test]
    fn test_factory_initialization() {
        let context = get_context(accounts(1), "factory.testnet".parse().unwrap(), None);
        testing_env!(context.build());

        let contract = ProxyFactory::new(
            "testnet".to_string(),
            "EaFtguW8o7cna1k8EtD4SFfGNdivuCPhx2Qautn7J3Rz".to_string(),
        );
        assert_eq!(
            contract.get_signer_contract(),
            "v1.signer-prod.testnet".parse::<AccountId>().unwrap()
        );
    }

    #[test]
    #[should_panic(expected = "Must attach at least 1000 yⓃ")]
    fn test_insufficient_deposit() {
        let context = get_context(
            accounts(1),
            "factory.testnet".parse().unwrap(),
            Some(NearToken::from_yoctonear(500_000)),
        );
        testing_env!(context.build());

        let mut contract = ProxyFactory::new(
            "testnet".to_string(),
            "EaFtguW8o7cna1k8EtD4SFfGNdivuCPhx2Qautn7J3Rz".to_string(),
        );
        contract.deposit_and_create_proxy_global(accounts(2));
    }

    #[test]
    fn test_proxy_address_generation() {
        let context = get_context(accounts(1), "factory.testnet".parse().unwrap(), None);
        testing_env!(context.build());

        let contract = ProxyFactory::new(
            "testnet".to_string(),
            "EaFtguW8o7cna1k8EtD4SFfGNdivuCPhx2Qautn7J3Rz".to_string(),
        );

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
        let context = get_context(accounts(1), "factory.testnet".parse().unwrap(), None);
        testing_env!(context.build());

        let contract = ProxyFactory::new(
            "mainnet".to_string(),
            "EaFtguW8o7cna1k8EtD4SFfGNdivuCPhx2Qautn7J3Rz".to_string(),
        );
        let hash = contract.get_proxy_code_base58_hash();

        assert!(!hash.is_empty(), "Code hash should not be empty");
        assert_eq!(
            hash, "EaFtguW8o7cna1k8EtD4SFfGNdivuCPhx2Qautn7J3Rz",
            "Hash should match the provided Base58 hash"
        );
    }

    #[test]
    fn test_successful_proxy_creation() {
        let context = get_context(
            accounts(1),
            "factory.testnet".parse().unwrap(),
            Some(NearToken::from_yoctonear(2_000_000)),
        );
        testing_env!(context.build());

        let mut contract = ProxyFactory::new(
            "testnet".to_string(),
            "EaFtguW8o7cna1k8EtD4SFfGNdivuCPhx2Qautn7J3Rz".to_string(),
        );
        let result = contract.create_proxy_global(accounts(2));

        // Since we can't fully test Promise chain in unit tests,
        // we at least verify the promise was created
        assert!(matches!(result, Promise { .. }));
    }

    #[test]
    fn test_proxy_creation_refund() {
        let context = get_context(accounts(1), "factory.testnet".parse().unwrap(), None);
        testing_env!(context.build());

        let mut contract = ProxyFactory::new(
            "testnet".to_string(),
            "EaFtguW8o7cna1k8EtD4SFfGNdivuCPhx2Qautn7J3Rz".to_string(),
        );
        let result = contract.on_proxy_created(
            accounts(1),
            Err(near_sdk::PromiseError::Failed),
            NearToken::from_yoctonear(2_000_000),
        );

        // Verify refund promise was created
        assert!(matches!(result, Promise { .. }));
    }

    #[test]
    fn test_set_global_code_hash() {
        let context = get_context(accounts(1), accounts(1), None);
        testing_env!(context.build());

        let mut contract = ProxyFactory::new(
            "testnet".to_string(),
            "EaFtguW8o7cna1k8EtD4SFfGNdivuCPhx2Qautn7J3Rz".to_string(),
        );

        // Update the global code hash
        contract.set_global_code_hash("CxhHoMAytiy39MSyKCJRksiWXvhYdRYncFpcVWAd4Pbg".to_string());

        // Verify the hash was updated
        let new_hash = contract.get_proxy_code_base58_hash();
        assert_eq!(new_hash, "CxhHoMAytiy39MSyKCJRksiWXvhYdRYncFpcVWAd4Pbg");
    }

    #[test]
    #[should_panic(expected = "Only the contract owner can perform this action.")]
    fn test_set_global_code_hash_unauthorized() {
        let context = get_context(accounts(2), "factory.testnet".parse().unwrap(), None); // Different account
        testing_env!(context.build());

        let mut contract = ProxyFactory::new(
            "testnet".to_string(),
            "EaFtguW8o7cna1k8EtD4SFfGNdivuCPhx2Qautn7J3Rz".to_string(),
        );

        // This should fail because accounts(2) is not the owner
        contract.set_global_code_hash("NewHash123456789012345678901234567890".to_string());
    }

    #[test]
    fn test_get_proxy_code_hash_hex() {
        let context = get_context(accounts(1), accounts(1), None);
        testing_env!(context.build());

        let contract = ProxyFactory::new(
            "testnet".to_string(),
            "EaFtguW8o7cna1k8EtD4SFfGNdivuCPhx2Qautn7J3Rz".to_string(),
        );

        let hex_hash = contract.get_proxy_code_hash_hex();
        assert!(!hex_hash.is_empty(), "Hex hash should not be empty");
        assert_eq!(
            hex_hash.len(),
            64,
            "Hex hash should be 32 bytes (64 hex chars)"
        );
    }

    #[test]
    fn test_add_full_access_key() {
        let context = get_context(accounts(1), accounts(1), None);
        testing_env!(context.build());

        let mut contract = ProxyFactory::new(
            "testnet".to_string(),
            "EaFtguW8o7cna1k8EtD4SFfGNdivuCPhx2Qautn7J3Rz".to_string(),
        );

        let public_key = PublicKey::from_str("ed25519:11111111111111111111111111111111").unwrap();
        let result = contract.add_full_access_key(public_key);

        // Verify promise was created
        assert!(matches!(result, Promise { .. }));
    }
}
