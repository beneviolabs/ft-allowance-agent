#[cfg(test)]
mod tests {
    use crate::{
        ActionString, AuthProxyContract, BigR, EcdsaSignatureResponse, EddsaSignatureResponse,
        ScalarValue, SignatureResponse,
    };
    use near_sdk::PublicKey;
    use near_sdk::{
        AccountId,
        json_types::{Base58CryptoHash, U64},
        test_utils::{VMContextBuilder, accounts},
        testing_env,
    };
    use omni_transaction::TxBuilder;
    use omni_transaction::near::utils::PublicKeyStrExt;
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
            accounts(3),
            "[{\"public_key\": \"ed25519:1234\"}]".to_string(),
            U64(1),
            Base58CryptoHash::from([0u8; 32]),
            "secp256k1:abcd".to_string(),
            "test_path".to_string(),
            None,
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
            accounts(3),
            actions_json.to_string(),
            U64(1),
            Base58CryptoHash::from([0u8; 32]),
            "secp256k1:abcd".to_string(),
            "ed25519:wxyz".to_string(),
            None,
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

        let result = contract.convert_deposits_to_strings(json_input).unwrap();

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

        let result = contract.convert_deposits_to_strings(json_input).unwrap();

        // Large numbers should be converted to strings (no scientific notation)
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

        let result = contract.convert_deposits_to_strings(json_input).unwrap();

        // All deposit values should be converted to strings (no scientific notation)
        let expected = format!(
            r#"{{"actions":[{{"deposit":"{}"}},{{"deposit":"{}"}},{{"deposit":"{}"}}]}}"#,
            large_number1, small_number, large_number2
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_request_signature_with_domain_id() {
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
                "type": "FunctionCall",
                "method_name": "ft_transfer_call",
                "args": {},
                "gas": "100000000000000",
                "deposit": "1000000000000000000000000"
            }
        ]"#;

        testing_env!(get_context(accounts(2)).build());
        let result = contract.request_signature(
            AccountId::try_from("wrap.near".to_string()).unwrap(),
            actions_json.to_string(),
            U64(1),
            Base58CryptoHash::from([0u8; 32]),
            "secp256k1:abcd".to_string(),
            "ed25519:wxyz".to_string(),
            Some(1),
        );

        // Test that the domain_id parameter is accepted (doesn't fail with parameter error)
        // The actual signing will fail due to invalid public key, but that's not what we're testing
        match result {
            Ok(_) => {
                // Test passed - the domain_id parameter was accepted
            }
            Err(error_msg) => {
                // Ensure the error is not related to the domain_id parameter
                assert!(
                    !error_msg.contains("domain_id"),
                    "Error should not be related to domain_id parameter: {}",
                    error_msg
                );
                // The error should be about invalid public key, which is expected
                assert!(
                    error_msg.contains("Invalid public key format"),
                    "Expected public key error, got: {}",
                    error_msg
                );
            }
        }
    }

    #[test]
    fn test_validate_and_build_actions_valid_function_call() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer.testnet".to_string()).unwrap(),
        );

        let actions = vec![ActionString::FunctionCall {
            method_name: "ft_transfer_call".to_string(),
            args: serde_json::json!({"receiver_id": "alice.near", "amount": "1000000000000000000000000"}),
            gas: "100000000000000".to_string(),
            deposit: "1000000000000000000000000".to_string(),
        }];

        let contract_id = AccountId::try_from("wrap.near".to_string()).unwrap();
        let result = contract.validate_and_build_actions(actions, &contract_id);

        assert!(result.is_ok());
        let omni_actions = result.unwrap();
        assert_eq!(omni_actions.len(), 1);
    }

    #[test]
    fn test_validate_and_build_actions_valid_transfer() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let actions = vec![ActionString::Transfer {
            deposit: "500000000000000000000000".to_string(),
        }];

        let contract_id = AccountId::try_from("wrap.near".to_string()).unwrap();
        let result = contract.validate_and_build_actions(actions, &contract_id);

        assert!(result.is_ok());
        let omni_actions = result.unwrap();
        assert_eq!(omni_actions.len(), 1);
    }

    #[test]
    fn test_validate_and_build_actions_disallowed_contract() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let actions = vec![ActionString::FunctionCall {
            method_name: "ft_transfer_call".to_string(),
            args: serde_json::json!({}),
            gas: "100000000000000".to_string(),
            deposit: "1000000000000000000000000".to_string(),
        }];

        let contract_id = AccountId::try_from("disallowed.near".to_string()).unwrap();
        let result = contract.validate_and_build_actions(actions, &contract_id);

        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("is not allowed"));
    }

    #[test]
    fn test_validate_and_build_actions_disallowed_method() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let actions = vec![ActionString::FunctionCall {
            method_name: "disallowed_method".to_string(),
            args: serde_json::json!({}),
            gas: "100000000000000".to_string(),
            deposit: "1000000000000000000000000".to_string(),
        }];

        let contract_id = AccountId::try_from("wrap.near".to_string()).unwrap();
        let result = contract.validate_and_build_actions(actions, &contract_id);

        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Method disallowed_method is restricted"));
    }

    #[test]
    fn test_validate_and_build_actions_invalid_gas_format() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let actions = vec![ActionString::FunctionCall {
            method_name: "ft_transfer_call".to_string(),
            args: serde_json::json!({}),
            gas: "invalid_gas".to_string(),
            deposit: "1000000000000000000000000".to_string(),
        }];

        let contract_id = AccountId::try_from("wrap.near".to_string()).unwrap();
        let result = contract.validate_and_build_actions(actions, &contract_id);

        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Invalid gas format"));
    }

    #[test]
    fn test_validate_and_build_actions_invalid_deposit_format() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let actions = vec![ActionString::FunctionCall {
            method_name: "ft_transfer_call".to_string(),
            args: serde_json::json!({}),
            gas: "100000000000000".to_string(),
            deposit: "invalid_deposit".to_string(),
        }];

        let contract_id = AccountId::try_from("wrap.near".to_string()).unwrap();
        let result = contract.validate_and_build_actions(actions, &contract_id);

        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Invalid deposit format"));
    }

    #[test]
    fn test_validate_and_build_actions_multiple_actions() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let actions = vec![
            ActionString::FunctionCall {
                method_name: "ft_transfer_call".to_string(),
                args: serde_json::json!({"receiver_id": "alice.near"}),
                gas: "100000000000000".to_string(),
                deposit: "1000000000000000000000000".to_string(),
            },
            ActionString::Transfer {
                deposit: "500000000000000000000000".to_string(),
            },
            ActionString::FunctionCall {
                method_name: "near_deposit".to_string(),
                args: serde_json::json!({}),
                gas: "50000000000000".to_string(),
                deposit: "0".to_string(),
            },
        ];

        let contract_id = AccountId::try_from("wrap.near".to_string()).unwrap();
        let result = contract.validate_and_build_actions(actions, &contract_id);

        assert!(result.is_ok());
        let omni_actions = result.unwrap();
        assert_eq!(omni_actions.len(), 3);
    }

    #[test]
    fn test_validate_and_build_actions_empty_actions() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer.testnet".to_string()).unwrap(),
        );

        let actions = vec![];

        let contract_id = AccountId::try_from("wrap.near".to_string()).unwrap();
        let result = contract.validate_and_build_actions(actions, &contract_id);

        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Actions cannot be empty"));
    }

    #[test]
    fn test_create_signature_request_with_domain_id() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer.testnet".to_string()).unwrap(),
        );

        // Create a simple mock transaction using the same pattern as in the main code
        let tx = omni_transaction::TransactionBuilder::new::<omni_transaction::NEAR>()
            .signer_id("test.near".to_string())
            .signer_public_key(
                "ed25519:11111111111111111111111111111111"
                    .to_public_key()
                    .unwrap(),
            )
            .nonce(1)
            .receiver_id("wrap.near".to_string())
            .block_hash(omni_transaction::near::types::BlockHash([0u8; 32]))
            .actions(vec![])
            .build();

        let result = contract.create_signature_request(
            &tx,
            "test.trading-account.near".to_string(),
            Some(1),
        );

        // Verify the result is valid JSON
        assert!(result.is_object());

        // Verify the structure contains the expected fields
        let request_obj = result.get("request").unwrap();
        assert!(request_obj.get("payload_v2").is_some());
        assert!(request_obj.get("path").is_some());
        assert!(request_obj.get("domain_id").is_some());

        // Verify specific values
        assert_eq!(
            request_obj.get("path").unwrap().as_str().unwrap(),
            "test.trading-account.near"
        );
        assert_eq!(request_obj.get("domain_id").unwrap(), 1);
    }

    #[test]
    fn test_create_signature_request_without_domain_id() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer.testnet".to_string()).unwrap(),
        );

        // Create a simple mock transaction using the same pattern as in the main code
        let tx = omni_transaction::TransactionBuilder::new::<omni_transaction::NEAR>()
            .signer_id("test.near".to_string())
            .signer_public_key(
                "ed25519:11111111111111111111111111111111"
                    .to_public_key()
                    .unwrap(),
            )
            .nonce(1)
            .receiver_id("wrap.near".to_string())
            .block_hash(omni_transaction::near::types::BlockHash([0u8; 32]))
            .actions(vec![])
            .build();

        let result =
            contract.create_signature_request(&tx, "test.trading-account.near".to_string(), None);

        // Verify the result is valid JSON
        assert!(result.is_object());

        // Verify the structure contains the expected fields
        let request_obj = result.get("request").unwrap();
        assert!(request_obj.get("payload_v2").is_some());
        assert!(request_obj.get("path").is_some());
        assert!(request_obj.get("domain_id").is_some());

        // Verify specific values
        assert_eq!(
            request_obj.get("path").unwrap().as_str().unwrap(),
            "test.trading-account.near"
        );
        assert_eq!(request_obj.get("domain_id").unwrap().as_u64().unwrap(), 0); // Should default to NEAR_MPC_DOMAIN_ID
    }

    #[test]
    fn test_create_signature_request_payload_structure() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer.testnet".to_string()).unwrap(),
        );

        // Create a simple mock transaction using the same pattern as in the main code
        let tx = omni_transaction::TransactionBuilder::new::<omni_transaction::NEAR>()
            .signer_id("test.near".to_string())
            .signer_public_key(
                "ed25519:11111111111111111111111111111111"
                    .to_public_key()
                    .unwrap(),
            )
            .nonce(1)
            .receiver_id("wrap.near".to_string())
            .block_hash(omni_transaction::near::types::BlockHash([0u8; 32]))
            .actions(vec![])
            .build();

        let result = contract.create_signature_request(
            &tx,
            "test.trading-account.near".to_string(),
            Some(1),
        );

        // Verify the payload_v2 structure
        let request_obj = result.get("request").unwrap();
        let payload_v2 = request_obj.get("payload_v2").unwrap();

        assert!(payload_v2.get("Ecdsa").is_some());

        // Verify Ecdsa field is a hex string
        let ecdsa = payload_v2.get("Ecdsa").unwrap().as_str().unwrap();
        assert!(!ecdsa.is_empty());
        assert!(ecdsa.chars().all(|c| c.is_ascii_hexdigit()));
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
        let deposits_values = [
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

        let result = contract.convert_deposits_to_strings(json_input).unwrap();

        // All deposit values should be converted to strings (no scientific notation)
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

        let result = contract.convert_deposits_to_strings(json_input).unwrap();

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

        let result = contract.convert_deposits_to_strings(json_input).unwrap();

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

        let result = contract.convert_deposits_to_strings(json_input).unwrap();

        // No changes should be made (field ordering may differ after JSON parse/serialize)
        assert!(result.contains(r#""other_field":"value""#));
        assert!(result.contains(r#""number":123"#));
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

        let result = contract.convert_deposits_to_strings(json_input).unwrap();

        // Zero should be converted to string
        assert_eq!(result, r#"{"deposit":"0"}"#);
    }

    #[test]
    fn test_calculate_gas_allocation_sufficient_gas() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer.testnet".to_string()).unwrap(),
        );

        // Test with sufficient gas (150 TGas)
        let attached_gas = near_sdk::Gas::from_tgas(150);
        let result = contract.calculate_gas_allocation(attached_gas);

        assert!(result.is_ok());
        let gas_for_signing = result.unwrap();

        // Should have 130 TGas for signing
        assert_eq!(gas_for_signing.as_tgas(), 130);
    }

    #[test]
    fn test_calculate_gas_allocation_exact_minimum() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        // Test with exact minimum gas (120 TGas: 100 for signing + 20 reserved)
        let attached_gas = near_sdk::Gas::from_tgas(120);
        let result = contract.calculate_gas_allocation(attached_gas);

        assert!(result.is_ok());
        let gas_for_signing = result.unwrap();

        assert_eq!(gas_for_signing.as_tgas(), 100);
    }

    #[test]
    fn test_calculate_gas_allocation_insufficient_gas() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer.testnet".to_string()).unwrap(),
        );

        // Test with insufficient gas (20 TGas: exactly reserved, 0 for signing)
        let attached_gas = near_sdk::Gas::from_tgas(20);
        let result = contract.calculate_gas_allocation(attached_gas);

        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        println!("Error message: {}", error_msg);
        assert!(error_msg.contains("Insufficient gas for signing"));
        assert!(error_msg.contains("Need at least"));
    }

    #[test]
    fn test_calculate_gas_allocation_very_low_gas() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        // Test with very low gas (5 TGas)
        let attached_gas = near_sdk::Gas::from_tgas(5);
        let result = contract.calculate_gas_allocation(attached_gas);

        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        println!("Error message: {}", error_msg);
        assert!(error_msg.contains("Insufficient gas for signing"));
    }

    #[test]
    fn test_calculate_gas_allocation_maximum_gas() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer.testnet".to_string()).unwrap(),
        );

        // Test with very high gas (1000 TGas)
        let attached_gas = near_sdk::Gas::from_tgas(1000);
        let result = contract.calculate_gas_allocation(attached_gas);

        assert!(result.is_ok());
        let gas_for_signing = result.unwrap();

        assert_eq!(gas_for_signing.as_tgas(), 980);
    }

    // ===== INPUT VALIDATION TESTS =====

    #[test]
    fn test_request_signature_invalid_public_key() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer.testnet".to_string()).unwrap(),
        );

        testing_env!(get_context(accounts(1)).build());
        contract.add_authorized_user(accounts(2));

        let actions_json = r#"[
            {
                "type": "FunctionCall",
                "method_name": "ft_transfer_call",
                "args": {},
                "gas": "100000000000000",
                "deposit": "1000000000000000000000000"
            }
        ]"#;

        testing_env!(get_context(accounts(2)).build());
        let result = contract.request_signature(
            AccountId::try_from("wrap.near".to_string()).unwrap(),
            actions_json.to_string(),
            U64(1),
            Base58CryptoHash::from([0u8; 32]),
            "invalid_public_key_format".to_string(),
            "m/44'/397'/0'/0'/0'".to_string(),
            Some(1),
        );

        assert!(result.is_err());
        match result {
            Err(_) => {
                // The error is a Promise, we can't easily extract the error message
                // but we know it should be an error due to invalid public key
            }
            Ok(_) => panic!("Expected error but got Ok"),
        }
    }

    #[test]
    fn test_request_signature_malformed_json() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        testing_env!(get_context(accounts(1)).build());
        contract.add_authorized_user(accounts(2));

        testing_env!(get_context(accounts(2)).build());
        let result = contract.request_signature(
            AccountId::try_from("wrap.near".to_string()).unwrap(),
            "invalid json {".to_string(),
            U64(1),
            Base58CryptoHash::from([0u8; 32]),
            "ed25519:11111111111111111111111111111111".to_string(),
            "m/44'/397'/0'/0'/0'".to_string(),
            Some(1),
        );

        assert!(result.is_err());
        match result {
            Err(_) => {
                // The error is a Promise, we can't easily extract the error message
                // but we know it should be an error due to malformed JSON
            }
            Ok(_) => panic!("Expected error but got Ok"),
        }
    }

    #[test]
    fn test_request_signature_invalid_gas_format() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer.testnet".to_string()).unwrap(),
        );

        testing_env!(get_context(accounts(1)).build());
        contract.add_authorized_user(accounts(2));

        let actions_json = r#"[
            {
                "type": "FunctionCall",
                "method_name": "ft_transfer_call",
                "args": {},
                "gas": "not_a_number",
                "deposit": "1000000000000000000000000"
            }
        ]"#;

        testing_env!(get_context(accounts(2)).build());
        let result = contract.request_signature(
            AccountId::try_from("wrap.near".to_string()).unwrap(),
            actions_json.to_string(),
            U64(1),
            Base58CryptoHash::from([0u8; 32]),
            "ed25519:11111111111111111111111111111111".to_string(),
            "m/44'/397'/0'/0'/0'".to_string(),
            Some(1),
        );

        assert!(result.is_err());
        match result {
            Err(_) => {
                // The error is a Promise, we can't easily extract the error message
                // but we know it should be an error due to invalid gas format
            }
            Ok(_) => panic!("Expected error but got Ok"),
        }
    }

    #[test]
    fn test_request_signature_invalid_deposit_format() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer.testnet".to_string()).unwrap(),
        );

        testing_env!(get_context(accounts(1)).build());
        contract.add_authorized_user(accounts(2));

        let actions_json = r#"[
            {
                "type": "FunctionCall",
                "method_name": "ft_transfer_call",
                "args": {},
                "gas": "100000000000000",
                "deposit": "not_a_number"
            }
        ]"#;

        testing_env!(get_context(accounts(2)).build());
        let result = contract.request_signature(
            AccountId::try_from("wrap.near".to_string()).unwrap(),
            actions_json.to_string(),
            U64(1),
            Base58CryptoHash::from([0u8; 32]),
            "ed25519:11111111111111111111111111111111".to_string(),
            "m/44'/397'/0'/0'/0'".to_string(),
            Some(1),
        );

        assert!(result.is_err());
        match result {
            Err(_) => {
                // The error is a Promise, we can't easily extract the error message
                // but we know it should be an error due to invalid deposit format
            }
            Ok(_) => panic!("Expected error but got Ok"),
        }
    }

    #[test]
    fn test_request_signature_insufficient_gas_attached() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        testing_env!(get_context(accounts(1)).build());
        contract.add_authorized_user(accounts(2));

        // Set up context with insufficient gas (50 TGas < 120 TGas required)
        let mut context = get_context(accounts(2));
        context.prepaid_gas(near_sdk::Gas::from_tgas(50));
        testing_env!(context.build());

        let actions_json = r#"[
            {
                "type": "FunctionCall",
                "method_name": "ft_transfer_call",
                "args": {},
                "gas": "100000000000000",
                "deposit": "1000000000000000000000000"
            }
        ]"#;

        let result = contract.request_signature(
            AccountId::try_from("wrap.near".to_string()).unwrap(),
            actions_json.to_string(),
            U64(1),
            Base58CryptoHash::from([0u8; 32]),
            "ed25519:11111111111111111111111111111111".to_string(),
            "m/44'/397'/0'/0'/0'".to_string(),
            Some(1),
        );

        // Should return an error due to insufficient gas
        assert!(result.is_err());
        match result {
            Err(error_msg) => {
                assert!(error_msg.contains("Insufficient gas for signing"));
            }
            Ok(_) => panic!("Expected error but got Ok"),
        }
    }

    // ===== SIGN REQUEST CALLBACK TESTS =====

    #[test]
    fn test_sign_request_callback_eddsa_success() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        // Create a mock transaction JSON
        let tx_json = r#"{
            "signer_id": "test.near",
            "signer_public_key": "ed25519:11111111111111111111111111111111",
            "nonce": 1,
            "receiver_id": "wrap.near",
            "block_hash": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
            "actions": []
        }"#;

        // Create a mock EDDSA signature response
        let signature_response = SignatureResponse::Eddsa(EddsaSignatureResponse {
            signature: vec![1; 64], // 64 bytes for ED25519
        });

        let result = contract.sign_request_callback(Ok(signature_response), tx_json.to_string());

        // Should return a base64 encoded transaction
        assert!(!result.is_empty());
        assert!(!result.starts_with("ERROR:"));
    }

    #[test]
    fn test_sign_request_callback_ecdsa_success() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        // Create a mock transaction JSON
        let tx_json = r#"{
            "signer_id": "test.near",
            "signer_public_key": "ed25519:11111111111111111111111111111111",
            "nonce": 1,
            "receiver_id": "wrap.near",
            "block_hash": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
            "actions": []
        }"#;

        // Create a mock ECDSA signature response
        let signature_response = SignatureResponse::Ecdsa(EcdsaSignatureResponse {
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

        let result = contract.sign_request_callback(Ok(signature_response), tx_json.to_string());

        // Should return a base64 encoded transaction
        assert!(!result.is_empty());
        assert!(!result.starts_with("ERROR:"));
    }

    #[test]
    fn test_sign_request_callback_parse_error() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let tx_json = r#"{
            "signer_id": "test.near",
            "signer_public_key": "ed25519:11111111111111111111111111111111",
            "nonce": 1,
            "receiver_id": "wrap.near",
            "block_hash": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
            "actions": []
        }"#;

        // Create a promise error
        let promise_error = near_sdk::PromiseError::Failed;
        let result = contract.sign_request_callback(Err(promise_error), tx_json.to_string());

        // Should return an error message
        assert!(result.starts_with("ERROR:"));
        assert!(result.contains("Failed to parse response JSON"));
    }

    #[test]
    fn test_sign_request_callback_invalid_transaction_json() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        // Invalid transaction JSON
        let invalid_tx_json = r#"{"invalid": "json"}"#;

        let signature_response = SignatureResponse::Eddsa(EddsaSignatureResponse {
            signature: vec![1; 64],
        });

        let result =
            contract.sign_request_callback(Ok(signature_response), invalid_tx_json.to_string());

        // Should return an error message
        assert!(result.starts_with("ERROR:"));
        assert!(result.contains("Failed to deserialize transaction"));
    }

    #[test]
    fn test_sign_request_callback_eddsa_invalid_length() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let tx_json = r#"{
            "signer_id": "test.near",
            "signer_public_key": "ed25519:11111111111111111111111111111111",
            "nonce": 1,
            "receiver_id": "wrap.near",
            "block_hash": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
            "actions": []
        }"#;

        // EDDSA signature with invalid length (not 64 bytes)
        let signature_response = SignatureResponse::Eddsa(EddsaSignatureResponse {
            signature: vec![1; 32], // Only 32 bytes, should be 64
        });

        let result = contract.sign_request_callback(Ok(signature_response), tx_json.to_string());

        // Should return an error message
        assert!(result.starts_with("ERROR:"));
        assert!(result.contains("Invalid ED25519 signature length"));
    }

    #[test]
    fn test_sign_request_callback_ecdsa_invalid_hex() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = AuthProxyContract::new(
            accounts(1),
            AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap(),
        );

        let tx_json = r#"{
            "signer_id": "test.near",
            "signer_public_key": "ed25519:11111111111111111111111111111111",
            "nonce": 1,
            "receiver_id": "wrap.near",
            "block_hash": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
            "actions": []
        }"#;

        // ECDSA signature with invalid hex
        let signature_response = SignatureResponse::Ecdsa(EcdsaSignatureResponse {
            scheme: "Secp256k1".to_string(),
            big_r: BigR {
                affine_point: "03INVALID_HEX_STRING".to_string(),
            },
            s: ScalarValue {
                scalar: "INVALID_HEX_STRING".to_string(),
            },
            recovery_id: 1,
        });

        let result = contract.sign_request_callback(Ok(signature_response), tx_json.to_string());

        // Should return an error message
        assert!(result.starts_with("ERROR:"));
        assert!(result.contains("Invalid hex"));
    }
}
