#[cfg(test)]
mod tests {

    use near_sdk::{
        json_types::{Base58CryptoHash, U64},
        test_utils::{accounts, VMContextBuilder},
        testing_env, AccountId, NearToken,
    };
    use proxy_contract::AuthProxyContract;
    use proxy_contract::MIN_DEPOSIT;

    fn get_context(predecessor: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .predecessor_account_id(predecessor)
            .attached_deposit(NearToken::from_yoctonear(MIN_DEPOSIT))
            .prepaid_gas(near_sdk::Gas::from_tgas(150));
        builder
    }

    #[test]
    fn test_new() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(accounts(1));
        assert_eq!(contract.get_owner_id(), accounts(1));
    }

    #[test]
    fn test_authorize_user() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(accounts(1));

        contract.add_authorized_user(accounts(2));
        assert!(contract.is_authorized(accounts(2)));
    }

    #[test]
    fn test_remove_authorized_user() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(accounts(1));

        contract.add_authorized_user(accounts(2));
        assert!(contract.is_authorized(accounts(2)));

        contract.remove_authorized_user(accounts(2));
        assert!(!contract.is_authorized(accounts(2)));
    }

    #[test]
    #[should_panic(expected = "You have no power here. Only the owner can perform this action.")]
    fn test_unauthorized_add_user() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(accounts(1));
        contract.add_authorized_user(accounts(3));
    }

    #[test]
    #[should_panic(expected = "You have no power here. Only the owner can perform this action.")]
    fn test_unauthorized_min_deposit_update() {
        let context = get_context(accounts(1));
        testing_env!(context.build());

        let mut contract = AuthProxyContract::new(accounts(0));
        contract.set_min_deposit(NearToken::from_near(10));
    }

    #[test]
    fn test_get_authorized_users() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(accounts(1));

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
        let mut contract = AuthProxyContract::new(accounts(1));

        contract.set_signer_contract(accounts(2));
        assert_eq!(contract.get_signer_contract(), accounts(2));
    }

    #[test]
    #[should_panic(expected = "You have no power here. Only the owner can perform this action.")]
    fn test_unauthorized_set_signer() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(accounts(1));
        contract.set_signer_contract(accounts(3));
    }

    #[test]
    #[should_panic(expected = "Unauthorized: only authorized users can request signatures")]
    fn test_unauthorized_request_signature() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(accounts(1));
        contract.request_signature(
            accounts(3),                                        // contract_id: AccountId
            "[{\"public_key\": \"ed25519:1234\"}]".to_string(), // actions_json: String
            U64(1),                                             // nonce: U64
            Base58CryptoHash::from([0u8; 32]),                  // block_hash: Base58CryptoHash
            "secp256k1:abcd".to_string(),                       // public_key: String
            "test_path".to_string(),                            // path: String
        );
    }

    #[test]
    #[should_panic(
        expected = "unknown variant `Sign Message`, expected `FunctionCall` or `Transfer`"
    )]
    fn test_disallowed_action() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(accounts(1));

        testing_env!(get_context(accounts(1)).build());
        contract.add_authorized_user(accounts(2));

        let actions_json = r#"[
            {
                "type": "Sign Message",
                "Message": "blah blah blah"
            }
        ]"#;

        testing_env!(get_context(accounts(2)).build());
        contract.request_signature(
            accounts(3),                       // contract_id
            actions_json.to_string(),          // actions_json
            U64(1),                            // nonce
            Base58CryptoHash::from([0u8; 32]), // block_hash
            "secp256k1:abcd".to_string(),      // public_key
            "ed25519:wxyz".to_string(),        // path
        );
    }

    #[test]
    fn test_factory_initialization() {
        let context = get_context(accounts(0));
        testing_env!(context.build());

        let contract = AuthProxyContract::new(accounts(0));

        assert_eq!(contract.get_owner_id(), accounts(0));
        assert_eq!(
            contract.get_min_deposit(),
            NearToken::from_yoctonear(MIN_DEPOSIT)
        );
        assert!(contract.get_authorized_users().is_empty());
    }

    #[test]
    #[should_panic(expected = "You have no power here. Only the owner can perform this action.")]
    fn test_unauthorized_update_proxy_code() {
        let context = get_context(accounts(1));
        testing_env!(context.build());

        let mut contract = AuthProxyContract::new(accounts(0));

        // Attempt to update proxy code as non-owner
        contract.update_proxy_code();
    }

    #[test]
    fn test_proxy_creation_callback() {
        let context = get_context(accounts(0));
        testing_env!(context.build());

        let mut contract = AuthProxyContract::new(accounts(0));
        let proxy_account: AccountId = "alice_agent.benevio-labs.testnet".parse().unwrap();

        // Test successful callback
        let success = contract.on_proxy_created(Ok(()), proxy_account.clone());
        assert!(success);

        // Test failed callback
        let failure = contract.on_proxy_created(Err(near_sdk::PromiseError::Failed), proxy_account);
        assert!(!failure);
    }

    #[test]
    fn test_min_deposit_management() {
        let context = get_context(accounts(0));
        testing_env!(context.build());

        let mut contract = AuthProxyContract::new(accounts(0));
        let new_min_deposit = NearToken::from_near(10);

        contract.set_min_deposit(new_min_deposit);
        assert_eq!(contract.get_min_deposit(), new_min_deposit);
    }
}
