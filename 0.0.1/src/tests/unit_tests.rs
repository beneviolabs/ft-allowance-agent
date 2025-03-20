#[cfg(test)]
mod tests {

    use near_sdk::{
        json_types::{U64,Base58CryptoHash}, test_utils::{accounts, VMContextBuilder}, testing_env, AccountId, NearToken
    };
    use proxy_contract::ProxyContract;

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
        assert_eq!(contract.get_owner_id(), accounts(1));
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
            "secp256k1:abcd".to_string(),
            "ed25519:wxyz".to_string(),
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
            "ft_transfer".to_string(),         // method_name
            vec![1, 2, 3],                     // args
            near_sdk::json_types::U64(10),                // gas
            NearToken::from_near(1),           // deposit
            U64(1),                            // nonce
            Base58CryptoHash::from([0u8; 32]), // block_hash
            "secp256k1:abcd".to_string(),
            "ed25519:wxyz".to_string(),
        );
    }
}
