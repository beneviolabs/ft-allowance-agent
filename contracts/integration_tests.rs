#[cfg(test)]
mod contract_tests {

    use anyhow::Result;
    use near_sdk::AccountId;
    use near_workspaces::{Account, Contract, DevNetwork, Worker};
    use serde_json::json;

    const WASM_FILEPATH: &[u8] = include_bytes!("target/near/proxy_contract.wasm");

    async fn init(worker: &Worker<impl DevNetwork>) -> Result<(Contract, Account)> {
        let proxy_contract = worker.dev_deploy(WASM_FILEPATH).await?;
        let owner = proxy_contract.as_account();

        // Initialize the contract
        let _result = proxy_contract
            .call("new")
            .args_json(json!({
                "owner_id": owner.id(),
                "signer_id": AccountId::try_from("v1.signer-prod.testnet".to_string()).unwrap()
            }))
            .transact()
            .await?;

        Ok((proxy_contract.clone(), owner.clone()))
    }

    #[tokio::test]
    async fn proxy_contract_initialization() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let (contract, owner) = init(&worker).await?;

        let contract_owner = contract
            .call("get_owner_id")
            .view()
            .await?
            .json::<String>()?;

        assert_eq!(
            contract_owner,
            owner.id().to_string(),
            "Contract owner should match"
        );

        // Test owner authorization
        let result = contract
            .call("is_authorized")
            .args_json(json!({
                "account_id": owner.id()
            }))
            .view()
            .await?
            .json::<bool>()?;

        assert!(result, "Owner should be authorized");

        Ok(())
    }

    #[tokio::test]
    async fn test_request_signature_unauthorized() -> Result<()> {
        let worker = near_workspaces::sandbox().await?;
        let (contract, _) = init(&worker).await?;

        // Create unauthorized user
        let unauthorized_user = worker.dev_create_account().await?;

        // Attempt signature request as unauthorized user
        let result = unauthorized_user
            .call(contract.id(), "request_signature")
            .args_json(json!({
                "contract_id": "wrap.testnet",
                "actions_json": "[{\"type\":\"FunctionCall\", \"deposit\": \"50000000000000000000000\", \"gas\": \"300000000000000\", \"method_name\": \"near_deposit\", \"args\": \"\"}]",
                "nonce": "1",
                "block_hash": "11111111111111111111111111111111",
                "mpc_signer_pk":"ed25519:asdf".to_string(),
                "derivation_path": "agent.auth-factory.appaccount.testnet".to_string(),

            }))
            .gas(near_workspaces::types::Gas::from_tgas(200))
            .transact()
            .await;

        println!("Result: {:?}", result);
        // Check status before unwrapping
        let is_ok = result.is_ok();
        // Unwrap the error since we expect this to fail
        let final_result = result.unwrap();
        assert!(is_ok);
        assert!(final_result.is_failure());
        let err_msg = format!("{:?}", final_result.failures());
        assert!(
            err_msg.contains("Unauthorized: only authorized users can request signatures"),
            "Expected 'Unauthorized:...' error, got: {}",
            err_msg
        );
        Ok(())
    }
}
