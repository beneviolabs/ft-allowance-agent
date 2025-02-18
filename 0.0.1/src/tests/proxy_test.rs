use near_workspaces::{operations::Function, Account, Contract, DevNetwork, Worker};
use serde_json::json;
use anyhow::Result;

const WASM_FILEPATH: &[u8] = include_bytes!("../target/wasm32-unknown-unknown/release/proxy_contract.wasm");

async fn init(worker: &Worker<impl DevNetwork>) -> Result<(Contract, Account)> {
    let proxy_contract = worker.dev_deploy(WASM_FILEPATH).await?;
    let owner = proxy_contract.as_account();

    // println!("Owner is : {:?}", owner);

    // Initialize the contract
    let _result = proxy_contract
        .call("new")
        .args_json(json!({
            "owner_id": owner.id()
        }))
        .transact()
        .await?;

    // println!("Contract initialized");

    Ok((proxy_contract.clone(), owner.clone()))
}

#[tokio::test]
async fn test_proxy_contract_initialization() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let (contract, owner) = init(&worker).await?;

    // Test getting the signer contract
    let signer = contract
        .call("get_signer_contract")
        .view()
        .await?
        .json::<String>()?;

    assert_eq!(signer, "v1.signer-prod.testnet");

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
async fn test_add_authorized_user() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let (contract, _owner) = init(&worker).await?;

    // Create a new account to authorize
    let new_user = worker.dev_create_account().await?;

    // Add new user as authorized user
    let _ = contract
        .call("add_authorized_user")
        .args_json(json!({
            "account_id": new_user.id()
        }))
        .transact()
        .await?;

    // Verify the user is authorized
    let is_authorized = contract
        .call("is_authorized")
        .args_json(json!({
            "account_id": new_user.id()
        }))
        .view()
        .await?
        .json::<bool>()?;

    assert!(is_authorized, "New user should be authorized");
    Ok(())
}

#[tokio::test]
async fn test_remove_authorized_user() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let (contract, _owner) = init(&worker).await?;

    // Create and authorize a new user
    let user = worker.dev_create_account().await?;
    let _ = contract
        .call("add_authorized_user")
        .args_json(json!({
            "account_id": user.id()
        }))
        .transact()
        .await?;

    // Remove authorization
    let _ = contract
        .call("remove_authorized_user")
        .args_json(json!({
            "account_id": user.id()
        }))
        .transact()
        .await?;

    // Verify user is no longer authorized
    let is_authorized = contract
        .call("is_authorized")
        .args_json(json!({
            "account_id": user.id()
        }))
        .view()
        .await?
        .json::<bool>()?;

    assert!(!is_authorized, "User should no longer be authorized");
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
            "contract_id": "test.near",
            "method_name": "test_method",
            "args": vec![1, 2, 3],
            "gas": "30000000000000",
            "deposit": "0",
            "nonce": "1",
            "block_hash": "11111111111111111111111111111111"
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
        "Expected 'Unauthorized:...' error, got: {}", err_msg
    );
    Ok(())
}

#[tokio::test]
async fn test_set_signer_contract() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let (contract, _owner) = init(&worker).await?;

    let new_signer = "new-signer.near".to_string();

    // Set new signer contract
    let _ = contract
        .call("set_signer_contract")
        .args_json(json!({
            "new_signer": new_signer
        }))
        .transact()
        .await?;

    // Verify new signer contract
    let current_signer = contract
        .call("get_signer_contract")
        .view()
        .await?
        .json::<String>()?;

    assert_eq!(current_signer, new_signer, "Signer contract should be updated");
    Ok(())
}

#[tokio::test]
async fn test_get_authorized_users() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let (contract, _owner) = init(&worker).await?;

    // Add multiple users
    let user1 = worker.dev_create_account().await?;
    let user2 = worker.dev_create_account().await?;

    let _ = contract
        .batch()
        .call(Function::new("add_authorized_user").args_json(json!({ "account_id": user1.id() })))
        .call(Function::new("add_authorized_user").args_json(json!({ "account_id": user2.id() })))
        .transact()
        .await?;

    // Get all authorized users
    let authorized_users = contract
        .call("get_authorized_users")
        .view()
        .await?
        .json::<Vec<String>>()?;

    assert!(authorized_users.contains(&user1.id().to_string()));
    assert!(authorized_users.contains(&user2.id().to_string()));
    Ok(())
}

