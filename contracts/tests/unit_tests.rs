#[cfg(test)]
mod tests {

    use near_sdk::{
        json_types::{Base58CryptoHash, U64},
        test_utils::{accounts, get_logs, VMContextBuilder},
        testing_env, AccountId, Gas, NearToken,
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
        let mut contract = ProxyContract::new(accounts(1));

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


    fn test_large_number_serialization() {
        let actions_json = r#"[{
            "type": "Transfer",
            "deposit": "1000000000000000000000000"
        }]"#;

        let context = get_context(accounts(1));
        testing_env!(context.build());

        let mut contract = ProxyContract::new(accounts(1));
        contract.add_authorized_user(accounts(2));

        testing_env!(get_context(accounts(2))
            .attached_deposit(NearToken::from_near(1))
            .prepaid_gas(Gas::from_tgas(1000))
            .predecessor_account_id(accounts(2))
            .build());

        contract.request_signature(
            accounts(2),
            actions_json.to_string(),
            U64(1),
            "11111111111111111111111111111111".try_into().unwrap(),
            "secp256k1:ZMPyNgKaUjsKzzQrJ2h2rMT8myKfSrGNNnBsuhA4uNFHpHy7bMq4BPuMzcbGy22hgmSK9cw8PfLqamwzHi7eGW4".to_string(),
            "ed25519:13mBWvPqTHeWCaBy5Roik3MhNLtbiFSJdPbJTUzDGR9h".to_string(),
        );

        // Check logs for properly formatted JSON
        let logs = get_logs();
        assert!(logs
            .iter()
            .any(|log| log.contains("\"2000000000000000000000000\"")));
    }
}
