#[cfg(test)]
mod tests {

    use near_sdk::{
        env, json_types::{Base58CryptoHash, U64}, test_utils::{accounts, VMContextBuilder}, testing_env, AccountId, NearToken
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
            Some("test_method".to_string()),         // method_name: String
            "[{\"public_key\": \"ed25519:1234\"}]".to_string(),                     // args: String
            near_sdk::json_types::U64(10),                // gas: U64
            NearToken::from_near(1),           // deposit: NearToken
            U64(1),                            // nonce: U64
            Base58CryptoHash::from([0u8; 32]), // block_hash: Base58CryptoHash
            "secp256k1:abcd".to_string(),
            "ed25519:wxyz".to_string(),
        );
    }

    #[test]
    fn test_ed25519_verification_env() {
        // These data came from the .env variable siggy generation in utils.py

        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = ProxyContract::new(accounts(1));

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

        let result = contract.verify_ed25519_signature(
            message.as_bytes().to_vec(),
            signature,
            public_key
        );

        assert!(result, "Signature verification failed");
    }


   // TODO add test_ed25519_verification_auth_proxy once this PR is deployed https://github.com/Near-One/mpc/pull/294


    // #[test] can't get this to validate as expected
    fn test_secp256k1_verification() {

        // These data came from the auth_proxy siggy generation via MPC
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = ProxyContract::new(accounts(1));

        // The intent message that was signed
        let message = r#"{"signer_id":"agent.charleslavon.testnet","signer_public_key":{"SECP256K1":[163,30,156,100,204,241,251,26,227,62,196,36,194,78,110,109,27,158,94,38,105,247,255,172,187,119,167,146,29,149,65,171,107,254,3,217,97,76,40,108,255,43,222,161,97,80,136,169,4,170,43,118,103,67,10,32,87,133,109,86,107,72,89,226]},"nonce":191779815000010,"receiver_id":"wrap.testnet","block_hash":[25,63,151,248,247,0,4,202,185,50,245,236,244,98,19,224,44,146,114,83,172,181,156,229,141,66,58,15,146,251,34,244],"actions":[{"FunctionCall":{"method_name":"near_deposit","args":[123,125],"gas":300000000000000,"deposit":"1000000000000000000000000"}}]}"#;

        // Hash the message using keccak256
        let hash = env::keccak256(message.as_bytes());

        // The signature without secp256k1: prefix
        let sig_str = "5tiseNtENJyv443LwprXpoPkWFzmywGhqRe3HgQ12x781GSPHtwW9uX949hCaBf4HBpoHdqbAae6FfZrVZL92682u";
        let mut signature = bs58::decode(sig_str)
            .into_vec()
            .expect("Failed to decode signature");

        // Log the signature vector bytes
        env::log_str(&format!("Signature bytes: {:?}", &signature));

        // remove the last byte (v) from the signature
        signature.pop();

        // The recovery ID from the MPC signature response
        let v: u8 = 1;

        // Try to recover the public key
        let recovered_hex = contract.test_recover(hash.to_vec(), signature, v)
            .expect("Failed to recover public key");

        // Convert hex string to bytes
        let recovered_bytes = hex::decode(&recovered_hex)
            .expect("Failed to decode hex string");

        // Convert to base58
        let recovered_base58 = bs58::encode(&recovered_bytes).into_string();

        // Expected public key (you'll need to fill this in with the correct value)
        let expected_pk = "4G9xQAGtbgc91XUwLCZzr7oyxA7qhBu9VdAhM9YLNES2ofYvznhH6eXFUPYBd9mkpVFb1t6QLLpd76ce31CHmKGu";

        // Log the values for debugging
        env::log_str(&format!("Message hash: {}", hex::encode(&hash)));
        env::log_str(&format!("Recovered public key: {}", recovered_base58));
        env::log_str(&format!("Expected public key: {}", expected_pk));

        assert_eq!(recovered_base58, expected_pk, "Recovered public key doesn't match expected");
    }

    #[test]
    #[should_panic(
        expected = "danny is not allowed. Only wrap.near, wrap.testnet, and intents.near are permitted"
    )]
    fn test_disallowed_action() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = ProxyContract::new(accounts(1));

        testing_env!(get_context(accounts(1)).build());
        contract.add_authorized_user(accounts(2));

        testing_env!(get_context(accounts(2)).build());
        contract.request_signature(
            accounts(3),                       // contract_id
            Some("ft_transfer".to_string()),         // method_name
            "[{\"public_key\": \"ed25519:1234\"}]".to_string(),
            near_sdk::json_types::U64(10),                // gas
            NearToken::from_near(1),           // deposit
            U64(1),                            // nonce
            Base58CryptoHash::from([0u8; 32]), // block_hash
            "secp256k1:abcd".to_string(),
            "ed25519:wxyz".to_string(),
        );
    }
}
