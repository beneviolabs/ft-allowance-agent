#[cfg(test)]
mod tests {
    use crate::{
        ActionString, AuthProxyContract, BigR, EcdsaSignatureResponse, EddsaSignatureResponse,
        ScalarValue, SignatureRequest, SignatureResponse,
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
        let _ = contract.request_signature(SignatureRequest {
            contract_id: accounts(3),
            actions_json: "[{\"public_key\": \"ed25519:1234\"}]".to_string(),
            nonce: U64(1),
            block_hash: Base58CryptoHash::from([0u8; 32]),
            mpc_signer_pk: "secp256k1:abcd".to_string(),
            derivation_path: "test_path".to_string(),
            domain_id: None,
        });
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
        let result = contract.request_signature(SignatureRequest {
            contract_id: accounts(3),
            actions_json: actions_json.to_string(),
            nonce: U64(1),
            block_hash: Base58CryptoHash::from([0u8; 32]),
            mpc_signer_pk: "secp256k1:abcd".to_string(),
            derivation_path: "ed25519:wxyz".to_string(),
            domain_id: None,
        });

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
        let result = contract.request_signature(SignatureRequest {
            contract_id: AccountId::try_from("wrap.near".to_string()).unwrap(),
            actions_json: actions_json.to_string(),
            nonce: U64(1),
            block_hash: Base58CryptoHash::from([0u8; 32]),
            mpc_signer_pk: "secp256k1:abcd".to_string(),
            derivation_path: "ed25519:wxyz".to_string(),
            domain_id: Some(1),
        });

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

        let request = SignatureRequest {
            contract_id: AccountId::try_from("wrap.near".to_string()).unwrap(),
            actions_json: "[]".to_string(),
            nonce: U64(1),
            block_hash: Base58CryptoHash::from([0u8; 32]),
            mpc_signer_pk:
                "ed25519:1234567890123456789012345678901234567890123456789012345678901234"
                    .to_string(),
            derivation_path: "test.trading-account.near".to_string(),
            domain_id: Some(1),
        };

        let result = contract.create_signature_request(
            &tx,
            request.derivation_path.clone(),
            request.domain_id,
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

        let request = SignatureRequest {
            contract_id: AccountId::try_from("wrap.near".to_string()).unwrap(),
            actions_json: "[]".to_string(),
            nonce: U64(1),
            block_hash: Base58CryptoHash::from([0u8; 32]),
            mpc_signer_pk:
                "ed25519:1234567890123456789012345678901234567890123456789012345678901234"
                    .to_string(),
            derivation_path: "test.trading-account.near".to_string(),
            domain_id: None,
        };

        let result = contract.create_signature_request(
            &tx,
            request.derivation_path.clone(),
            request.domain_id,
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

        let request = SignatureRequest {
            contract_id: AccountId::try_from("wrap.near".to_string()).unwrap(),
            actions_json: "[]".to_string(),
            nonce: U64(1),
            block_hash: Base58CryptoHash::from([0u8; 32]),
            mpc_signer_pk:
                "ed25519:1234567890123456789012345678901234567890123456789012345678901234"
                    .to_string(),
            derivation_path: "test.trading-account.near".to_string(),
            domain_id: Some(1),
        };

        let result = contract.create_signature_request(
            &tx,
            request.derivation_path.clone(),
            request.domain_id,
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
}
