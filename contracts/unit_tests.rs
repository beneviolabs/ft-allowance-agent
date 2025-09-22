#[cfg(test)]
mod tests {
    use crate::{
        AuthProxyContract, BigR, EcdsaSignatureResponse, EddsaSignatureResponse, ScalarValue,
        SignatureResponse,
    };
    use near_sdk::PublicKey;
    use near_sdk::{
        AccountId,
        json_types::{Base58CryptoHash, U64},
        test_utils::{VMContextBuilder, accounts},
        testing_env,
    };
    use std::str::FromStr;

    fn get_context(predecessor: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .predecessor_account_id(predecessor)
            .prepaid_gas(near_sdk::Gas::from_tgas(150));
        builder
    }

    #[test]
    fn test_new() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );
        assert_eq!(contract.get_owner_id(), accounts(1));
    }

    #[test]
    fn test_authorize_user() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        contract.add_authorized_user(accounts(2));
        assert!(contract.is_authorized(accounts(2)));
    }

    #[test]
    fn test_remove_authorized_user() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer".to_string()).unwrap(),
        );

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
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );
        contract.add_authorized_user(accounts(3));
    }

    #[test]
    fn test_get_authorized_users() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        contract.add_authorized_user(accounts(2));
        contract.add_authorized_user(accounts(3));

        let users = contract.get_authorized_users();
        assert_eq!(users.len(), 2);
        assert!(users.contains(&accounts(2)));
        assert!(users.contains(&accounts(3)));
    }

    #[test]
    #[should_panic(expected = "Unauthorized: only authorized users can request signatures")]
    fn test_unauthorized_request_signature() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );
        let _ = contract.request_signature(
            accounts(3),                                        // contract_id: AccountId
            "[{\"public_key\": \"ed25519:1234\"}]".to_string(), // actions_json: String
            U64(1),                                             // nonce: U64
            Base58CryptoHash::from([0u8; 32]),                  // block_hash: Base58CryptoHash
            "secp256k1:abcd".to_string(),                       // public_key: String
            "test_path".to_string(),                            // path: String
        );
    }

    #[test]
    fn test_disallowed_action() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        testing_env!(get_context(accounts(1)).build());
        contract.add_authorized_user(accounts(2));

        let actions_json = r#"[
            {
                "type": "Sign Message",
                "Message": "blah blah blah"
            }
        ]"#;

        testing_env!(get_context(accounts(2)).build());
        let result = contract.request_signature(
            accounts(3),                       // contract_id
            actions_json.to_string(),          // actions_json
            U64(1),                            // nonce
            Base58CryptoHash::from([0u8; 32]), // block_hash
            "secp256k1:abcd".to_string(),      // public_key
            "ed25519:wxyz".to_string(),        // path
        );

        // Assert that the function returns an error for invalid action type
        assert!(result.is_err());
        match result {
            Err(error_msg) => {
                assert!(error_msg.contains("unknown variant `Sign Message`"));
            }
            Ok(_) => panic!("Expected error but got Ok"),
        }
    }

    #[test]
    fn test_signature_response_serialization() {
        // Test EDDSA signature response
        let eddsa_response = SignatureResponse::Eddsa(EddsaSignatureResponse {
            signature: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        });

        let json = serde_json::to_string(&eddsa_response).unwrap();
        let decoded: SignatureResponse = serde_json::from_str(&json).unwrap();

        match decoded {
            SignatureResponse::Eddsa(eddsa) => {
                assert_eq!(eddsa.signature, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
            }
            _ => panic!("Expected EDDSA signature response"),
        }

        // Test ECDSA signature response
        let ecdsa_response = SignatureResponse::Ecdsa(EcdsaSignatureResponse {
            scheme: "Secp256k1".to_string(),
            big_r: BigR {
                affine_point: "03D0E412BEEBF4B0191C08E13323466A96582C95A2B0BAF4CB6859968B86C01157"
                    .to_string(),
            },
            s: ScalarValue {
                scalar: "1AE54A1E7D404FD655B43C05DA78D1A6DC5ABAC2AE2A8338F03580D14A2C17F9"
                    .to_string(),
            },
            recovery_id: 1,
        });

        let json = serde_json::to_string(&ecdsa_response).unwrap();
        let decoded: SignatureResponse = serde_json::from_str(&json).unwrap();

        match decoded {
            SignatureResponse::Ecdsa(ecdsa) => {
                assert_eq!(ecdsa.scheme, "Secp256k1");
                assert_eq!(ecdsa.recovery_id, 1);
            }
            _ => panic!("Expected ECDSA signature response"),
        }
    }

    #[test]
    fn test_add_full_access_key_owner() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );
        let pk = PublicKey::from_str("ed25519:11111111111111111111111111111111").unwrap();
        let result = contract.add_full_access_key(pk);
        // We can't fully test Promise chain, but we can check the type
        assert!(matches!(result, near_sdk::Promise { .. }));
    }

    #[test]
    #[should_panic(expected = "You have no power here. Only the owner can perform this action.")]
    fn test_add_full_access_key_non_owner() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );
        let pk = PublicKey::from_str("ed25519:11111111111111111111111111111111").unwrap();
        contract.add_full_access_key(pk);
    }

    #[test]
    fn test_add_full_access_key_and_register_with_intents_owner() {
        let mut context = get_context(accounts(1));
        context.attached_deposit(near_sdk::NearToken::from_yoctonear(1));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );
        let pk = PublicKey::from_str("ed25519:11111111111111111111111111111111").unwrap();
        let result = contract.add_full_access_key_and_register_with_intents(pk);
        assert!(matches!(result, near_sdk::Promise { .. }));
    }

    #[test]
    #[should_panic(expected = "You have no power here. Only the owner can perform this action.")]
    fn test_add_full_access_key_and_register_with_intents_non_owner() {
        let mut context = get_context(accounts(2));
        context.attached_deposit(near_sdk::NearToken::from_yoctonear(1));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );
        let pk = PublicKey::from_str("ed25519:11111111111111111111111111111111").unwrap();
        contract.add_full_access_key_and_register_with_intents(pk);
    }

    #[test]
    fn test_convert_deposits_to_strings_small_numbers() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let json_input = r#"{"deposit":1000000,"other_field":"value"}"#.to_string();
        let deposits = vec![omni_transaction::near::types::U128(1000000)];

        let result = contract.convert_deposits_to_strings(json_input, &deposits);

        // All deposit values should be converted to strings
        assert_eq!(result, r#"{"deposit":"1000000","other_field":"value"}"#);
    }

    #[test]
    fn test_convert_deposits_to_strings_large_numbers() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let large_number = 10_000_000_000_000_000_000_000u128; // Larger than MAX_SAFE_INTEGER
        let json_input = format!(r#"{{"deposit":{},"other_field":"value"}}"#, large_number);
        let deposits = vec![omni_transaction::near::types::U128(large_number)];

        let result = contract.convert_deposits_to_strings(json_input, &deposits);

        // Large numbers should be converted to strings
        assert_eq!(
            result,
            format!(r#"{{"deposit":"{}","other_field":"value"}}"#, large_number)
        );
    }

    #[test]
    fn test_convert_deposits_to_strings_multiple_deposits() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let large_number1 = 10_000_000_000_000_000_000_000u128;
        let large_number2 = 20_000_000_000_000_000_000_000u128;
        let small_number = 1000000u128;

        let json_input = format!(
            r#"{{"actions":[{{"deposit":{}}},{{"deposit":{}}},{{"deposit":{}}}]}}"#,
            large_number1, small_number, large_number2
        );
        let deposits = vec![
            omni_transaction::near::types::U128(large_number1),
            omni_transaction::near::types::U128(small_number),
            omni_transaction::near::types::U128(large_number2),
        ];

        let result = contract.convert_deposits_to_strings(json_input, &deposits);

        // All deposit values should be converted to strings
        let expected = format!(
            r#"{{"actions":[{{"deposit":"{}"}},{{"deposit":"{}"}},{{"deposit":"{}"}}]}}"#,
            large_number1, small_number, large_number2
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_convert_deposits_to_strings_ten_distinct_values() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        // Create 10 distinct deposit values
        let deposits_values = vec![
            0u128,
            1u128,
            1000u128,
            1000000u128,
            1000000000u128,
            1000000000000u128,
            1000000000000000u128,
            1000000000000000000u128,
            1000000000000000000000u128,
            1000000000000000000000000u128,
        ];

        // Create JSON with all deposit values
        let json_input = format!(
            r#"{{"actions":[{{"deposit":{}}},{{"deposit":{}}},{{"deposit":{}}},{{"deposit":{}}},{{"deposit":{}}},{{"deposit":{}}},{{"deposit":{}}},{{"deposit":{}}},{{"deposit":{}}},{{"deposit":{}}}]}}"#,
            deposits_values[0],
            deposits_values[1],
            deposits_values[2],
            deposits_values[3],
            deposits_values[4],
            deposits_values[5],
            deposits_values[6],
            deposits_values[7],
            deposits_values[8],
            deposits_values[9]
        );

        let deposits: Vec<_> = deposits_values
            .iter()
            .map(|&v| omni_transaction::near::types::U128(v))
            .collect();

        let result = contract.convert_deposits_to_strings(json_input, &deposits);

        // All deposit values should be converted to strings and maintain their order
        let expected = format!(
            r#"{{"actions":[{{"deposit":"{}"}},{{"deposit":"{}"}},{{"deposit":"{}"}},{{"deposit":"{}"}},{{"deposit":"{}"}},{{"deposit":"{}"}},{{"deposit":"{}"}},{{"deposit":"{}"}},{{"deposit":"{}"}},{{"deposit":"{}"}}]}}"#,
            deposits_values[0],
            deposits_values[1],
            deposits_values[2],
            deposits_values[3],
            deposits_values[4],
            deposits_values[5],
            deposits_values[6],
            deposits_values[7],
            deposits_values[8],
            deposits_values[9]
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_convert_deposits_to_strings_edge_case_max_safe_integer() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let max_safe_integer = 9_007_199_254_740_991u128; // JavaScript's MAX_SAFE_INTEGER
        let json_input = format!(r#"{{"deposit":{}}}"#, max_safe_integer);
        let deposits = vec![omni_transaction::near::types::U128(max_safe_integer)];

        let result = contract.convert_deposits_to_strings(json_input, &deposits);

        // All deposit values should be converted to strings
        assert_eq!(result, format!(r#"{{"deposit":"{}"}}"#, max_safe_integer));
    }

    #[test]
    fn test_convert_deposits_to_strings_edge_case_max_safe_integer_plus_one() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let max_safe_integer_plus_one = 9_007_199_254_740_992u128; // MAX_SAFE_INTEGER + 1
        let json_input = format!(r#"{{"deposit":{}}}"#, max_safe_integer_plus_one);
        let deposits = vec![omni_transaction::near::types::U128(
            max_safe_integer_plus_one,
        )];

        let result = contract.convert_deposits_to_strings(json_input, &deposits);

        // All deposit values should be converted to strings
        assert_eq!(
            result,
            format!(r#"{{"deposit":"{}"}}"#, max_safe_integer_plus_one)
        );
    }

    #[test]
    fn test_convert_deposits_to_strings_no_deposits() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let json_input = r#"{"other_field":"value","number":123}"#.to_string();
        let deposits = vec![];
        let expected = json_input.clone();

        let result = contract.convert_deposits_to_strings(json_input, &deposits);

        // No changes should be made
        assert_eq!(result, expected);
    }

    #[test]
    fn test_convert_deposits_to_strings_zero_deposit() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let json_input = r#"{"deposit":0}"#.to_string();
        let deposits = vec![omni_transaction::near::types::U128(0)];

        let result = contract.convert_deposits_to_strings(json_input, &deposits);

        // Zero should be converted to string
        assert_eq!(result, r#"{"deposit":"0"}"#);
    }
}
