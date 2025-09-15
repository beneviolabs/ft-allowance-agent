# Global Contract Workflow Documentation

This document outlines the complete process for building, deploying, and using global contracts with the Auth Proxy Factory contracts.

## Overview

The global contract feature (NEP-591) allows deploying contract code once globally and reusing it across multiple accounts, significantly reducing storage costs from ~3.8 NEAR to ~0.001 NEAR per trading account.

## Prerequisites

- NEAR CLI installed (`npm install -g near-cli`)
- Rust toolchain with wasm32-unknown-unknown target
- near's cargo extension to support running: `./contracts/build_auth_proxy.sh` and `/contracts/factory/factory-deploy.sh`

## 1. Build and Globally Deploy Auth Proxy Contract

### Step 1.1: Build the Auth Proxy Contract

```bash
# Navigate to the contracts directory
cd contracts

# Build the auth proxy contract
./build_auth_proxy.sh
```

Notice the bs58 hash in the output
```

     - SHA-256 checksum hex : c9acef2aaaab73d07684b79b4655f7ab946c7e15e3091f1fae6bc5df86566bd9
     ** - SHA-256 checksum bs58: EaFtguW8o7cna1k8EtD4SFfGNdivuCPhx2Qautn7J3Rz **
    Finished cargo near build in 19s
```
This bs58 hash should equal the code_hash returned from the near-cli-rs global deploy transaction.

### Step 1.2: Deploy as Global Contract

```bash
# Deploy the auth proxy code globally
near contract deploy-as-global \
  use-file ft-allowance/contracts/target/near/proxy_contract.wasm \
  as-global-hash base-account.testnet \
  network-config testnet
```

**Expected Output:**
- A Base58-encoded global contract hash (e.g., `EaFtguW8o7cna1k8EtD4SFfGNdivuCPhx2Qautn7J3Rz`)
- Save this hash for the factory deployment


## 2. Deploy A Factory Contract

### Step 2.1: Update Deployment Script

Edit `contracts/factory/factory-deploy.sh`:

```bash
# Set the global contract hash from step 1.2
GLOBAL_PROXY_CODE_HASH="EaFtguW8o7cna1k8EtD4SFfGNdivuCPhx2Qautn7J3Rz"

# Set the factory account
FACTORY_ACCOUNT="auth-v1.base-account.testnet"
FACTORY_OWNER="base-account.testnet"
```

### Step 2.2: Deploy Factory Contract

```bash
# Navigate to factory directory
cd contracts/factory

# Run the deployment script
NEAR_ENV=testnet ./factory-deploy.sh
```

**What happens:**
1. Builds the factory contract with global-contracts feature enabled
2. Creates the factory account, auth-v1.base-account.testnet if it doesn't exist
3. Deploys the factory with the global contract hash
4. Verifies deployment and checksums

### Step 2.3: Verify Factory Deployment

The base58 hash output from the call below should match the hash output from the global deploy transaction, and match the bs58 output if you re-run `./build_auth_proxy.sh`. There is no harm in running that command multiple times.

```bash
# View the stored global contract hash
near call auth-v1.base-account.testnet get_proxy_code_base58_hash --accountId base-account.testnet
```


## 3. Update an Existing Factory Contract

### When to Update vs Deploy New Factory

**Update Existing Factory When:**
- Global contract hash changes (to point to a new proxy code version)
- Factory logic improvements (e.g. bug fixes)
- Configuration changes (new signer contracts)
- Gas optimizations on factory methods

**Deploy New Factory When:**
- A state change, e.g. changes to the ProxyFactory struct
- Migration to different account structure
- Major version upgrades

### Step 3.1: Update Factory to point to the latest Global Proxy Contract Hash

If you've deployed a new version of the auth proxy globally:

```bash
# Update the factory with new base58 encoded global contract hash
near call auth-v1.base-account.testnet set_global_code_hash \
  '{"code_hash_str": "NEW_GLOBAL_HASH_HERE"}' \
  --accountId base-account.testnet
```

### Step 3.2: Update Factory Contract Code

For factory code updates:

```bash
# Navigate to factory directory
cd contracts/factory

# Run deployment script (will update existing contract)
NEAR_ENV=testnet ./factory-deploy.sh
```

**Note:** The script automatically detects if the account exists and updates the contract instead of creating a new one.

### Step 3.3: Verify Update

```bash
# Check the updated global hash via get_proxy_code_hash_hex or get_proxy_code_base58_hash
near call auth-v1.base-account.testnet get_proxy_code_base58_hash --accountId base-account.testnet


## 4. Create Trading Account via Global Contract

### Step 4.1: Create Proxy Account

```bash
# Create a new proxy account using the global contract
near call auth-v1.peerfolio.testnet deposit_and_create_proxy_global \
  '{"owner_id": "trader.peerfolio.testnet"}' \
  --accountId trader.peerfolio.testnet \
  --deposit 0.001
```

**Expected Result:**
- Creates sub-account: `trader.auth-v1.peerfolio.testnet`
- Uses global contract code (no individual deployment)
- Costs only ~0.001 NEAR instead of ~3.8 NEAR

### Step 4.2: Verify Proxy Creation

```bash
# Check the created proxy account
near state trader.auth-v1.peerfolio.testnet

# Verify it's Global Contract (by Hash: SHA-256 checksum hex) matches the factory's hex hash of the bs58 code
near call auth-v1.peerfolio.testnet get_proxy_code_hash_hex '{}'
```

## Migration Strategy

If migrating from traditionally deployed trading account to global contracts:

1. Since you're already "paying" for your contract's code to be stored onchain, you deploy the latest auth proxy wasm onto your trading account. A requisite is that you have added a full access key onto your trading account, then `near contract deploy charleslavon.auth-v0.peerfolio.near use-file /contracts/target/near/proxy_contract.wasm without-init-call network-config mainnet sign-with-plaintext-private-key`
