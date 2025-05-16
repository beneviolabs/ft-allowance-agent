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
    fn test_ed25519_verification_env() {
        // These data came from the .env variable siggy generation in utils.py

        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(accounts(1));

        // Your intent message
        let message = "{\"signer_id\": \"charleslavon.near\", \"nonce\": \"5x9D1/ppzzCfGyDM6kjeIl560bbc2pvLMu+rIeiKyHE=\", \"verifying_contract\": \"intents.near\", \"deadline\": \"2025-04-02T18:58:10.000Z\", \"intents\": [{\"intent\": \"token_diff\", \"diff\": {\"nep141:wrap.near\": \"-1000000000000000000000000\", \"nep141:usdt.tether-token.near\": \"2642656\"}, \"referral\": \"benevio-labs.near\"}]}";

        // Convert base58 signature to bytes
        let sig_str = "4mLRJJi4hAAyuKJTq4RX3997WPbbfxPaEiw8snS96V4DPQre6iYLMwJWWw6VwftP3Y8g4qjDsNa5xLn3MBsYBfLg";
        let signature = bs58::decode(sig_str)
            .into_vec()
            .expect("Failed to decode signature");

        // Your public key in bytes
        let public_key = bs58::decode("9RqZPDhjgQQDFTpREQqnasYuM1FKKrKpHDWxPJaeJGYb")
            .into_vec()
            .expect("Failed to decode public key");

        let result =
            contract.verify_ed25519_signature(message.as_bytes().to_vec(), signature, public_key);

        assert!(result, "Signature verification failed");
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
    fn test_create_proxy() {
        let context = get_context(accounts(0));
        testing_env!(context.build());

        let mut contract = AuthProxyContract::new(accounts(0));

        // Set proxy code
        let proxy_code = vec![1, 2, 3, 4]; // Mock WASM bytes
        contract.update_proxy_code(proxy_code.clone());

        // Create proxy account
        let proxy_account: AccountId = "alice_agent.benevio-labs.testnet".parse().unwrap();
        let _result = contract.create_proxy(proxy_account.clone());

        // No assertion needed as create_proxy returns a Promise
        // If it didn't panic, the Promise was created successfully
    }

    #[test]
    #[should_panic(expected = "Not enough deposit")]
    fn test_create_proxy_insufficient_deposit() {
        let context = get_context(accounts(0))
            .attached_deposit(NearToken::from_near(0))
            .build();
        testing_env!(context);

        let mut contract = AuthProxyContract::new(accounts(0));
        let proxy_account: AccountId = "alice_agent.benevio-labs.testnet".parse().unwrap();
        contract.create_proxy(proxy_account);
    }

    #[test]
    fn test_update_proxy_code() {
        let context = get_context(accounts(0));
        testing_env!(context.build());

        let mut contract = AuthProxyContract::new(accounts(0));
        let proxy_code = vec![1, 2, 3, 4];

        contract.update_proxy_code(proxy_code.clone());
        // Note: We can't directly verify proxy_code as it's private
        // Instead we verify through create_proxy functionality
        let proxy_account: AccountId = "alice_agent.benevio-labs.testnet".parse().unwrap();
        contract.create_proxy(proxy_account);
    }

    #[test]
    #[should_panic(expected = "You have no power here. Only the owner can perform this action.")]
    fn test_unauthorized_update_proxy_code() {
        let context = get_context(accounts(1));
        testing_env!(context.build());

        let mut contract = AuthProxyContract::new(accounts(0));
        contract.update_proxy_code(vec![1, 2, 3, 4]);
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
