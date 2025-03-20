#!/bin/bash

# Check if NEAR CLI is installed
if ! command -v near &> /dev/null; then
    echo "NEAR CLI is not installed. Please install it first with: npm install -g near-cli"
    exit 1
fi

# Clear previous builds
echo "Clearing previous builds..."
cargo clean

# Build the contract
echo "Building contract..."
env RUSTFLAGS='-Ctarget-cpu=mvp' cargo +nightly build -Zbuild-std=panic_abort,std --target=wasm32-unknown-unknown --release

# Set variables
WASM_PATH="../target/wasm32-unknown-unknown/release/proxy_contract.wasm"

# Check if WASM file exists
if [ ! -f "$WASM_PATH" ]; then
    echo "Error: WASM file not found at $WASM_PATH"
    exit 1
fi

# Check if AGENT_PARENT_ACCOUNT is set
if [ -z "$AGENT_PARENT_ACCOUNT" ]; then
    echo "Error: AGENT_PARENT_ACCOUNT environment variable is not set"
    exit 1
fi

# Check if contract needs initialization
echo "Checking agent proxy account state..."
CONTRACT_STATE=$(near state $AGENT_PROXY_ACCOUNT)
CONTRACT_INITIALIZATION_REQUIRED=false

if echo "$CONTRACT_STATE" | grep -q "No contract code"; then
    CONTRACT_INITIALIZATION_REQUIRED=true
    echo "Contract will be initialized during deployment"
else
    echo "Contract already initialized, its code will be updated"
fi

# Deploy the contract
echo "Deploying contract to $AGENT_PROXY_ACCOUNT..."
if [ "$CONTRACT_INITIALIZATION_REQUIRED" = true ]; then
    echo "Deploying contract with initialization..."
    near contract deploy "$AGENT_PROXY_ACCOUNT" \
        use-file "$WASM_PATH" \
        with-init-call new \
        json-args '{"owner_id":"'"$AGENT_PARENT_ACCOUNT"'"}' \
        prepaid-gas '100.0 Tgas' \
        attached-deposit '0 NEAR' \
        network-config testnet \
        sign-with-keychain \
        send
else
    echo "Deploying contract without initialization..."
    near contract deploy "$AGENT_PROXY_ACCOUNT" \
        use-file "$WASM_PATH" \
        without-init-call \
        network-config testnet \
        sign-with-keychain \
        send
fi

# Check deployment status
if [ $? -eq 0 ]; then
    echo "Contract deployed successfully to $AGENT_PROXY_ACCOUNT"
else
    echo "Contract deployment failed"
    exit 1
fi
