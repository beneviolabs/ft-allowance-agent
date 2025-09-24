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
    #[should_panic(
        expected = "Transfer actions must be accompanied by at least one FunctionCall action"
    )]
    fn test_request_signature_single_transfer_action_fails() {
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
                "type": "Transfer",
                "deposit": "1000000000000000000000000"
            }
        ]"#;

        testing_env!(get_context(accounts(2)).build());
        contract.request_signature(
            AccountId::try_from("bad-account.near".to_string()).unwrap(),
            actions_json.to_string(),
            U64(1),
            Base58CryptoHash::from([0u8; 32]),
            "ed25519:11111111111111111111111111111111".to_string(),
            "trading-account.near".to_string(),
        );
    }

    #[test]
    #[should_panic(expected = "GasExceeded")]
    fn test_request_signature_multiple_actions_with_transfer_succeeds() {
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
                "args": {"receiver_id": "alice.near", "amount": "1000000000000000000000000"},
                "gas": "100000000000000",
                "deposit": "1000000000000000000000000"
            },
            {
                "type": "Transfer",
                "deposit": "1000000000000000000000000"
            }
        ]"#;

        testing_env!(get_context(accounts(2)).build());
        let _result = contract.request_signature(
            AccountId::try_from("wrap.near".to_string()).unwrap(),
            actions_json.to_string(),
            U64(1),
            Base58CryptoHash::from([0u8; 32]),
            "ed25519:11111111111111111111111111111111".to_string(),
            "trading-account.near".to_string(),
        );
        // Test passes  - gas exceeded - but validation succeeds
    }

    #[test]
    #[should_panic(
        expected = "Transfer actions must be accompanied by at least one FunctionCall action"
    )]
    fn test_request_signature_multiple_transfer_actions_without_function_call_fails() {
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
                "type": "Transfer",
                "deposit": "1000000000000000000000000"
            },
            {
                "type": "Transfer",
                "deposit": "2000000000000000000000000"
            }
        ]"#;

        testing_env!(get_context(accounts(2)).build());
        contract.request_signature(
            AccountId::try_from("wrap.near".to_string()).unwrap(),
            actions_json.to_string(),
            U64(1),
            Base58CryptoHash::from([0u8; 32]),
            "ed25519:11111111111111111111111111111111".to_string(),
            "trading-account.near".to_string(),
        );
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
}
