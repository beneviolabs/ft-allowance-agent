#!/bin/bash
# filepath: /Users/charles/.nearai/registry/charleslavon.near/ft-allowance/0.0.1/src/scripts/deploy.sh

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
CONTRACT_ID="1.charleslavon.testnet"
WASM_PATH="target/wasm32-unknown-unknown/release/proxy_contract.wasm"

# Check if WASM file exists
if [ ! -f "$WASM_PATH" ]; then
    echo "Error: WASM file not found at $WASM_PATH"
    exit 1
fi

# Deploy the contract
echo "Deploying contract to $CONTRACT_ID..."
near contract deploy "$CONTRACT_ID" \
    use-file "$WASM_PATH" \
    without-init-call \
    network-config testnet \
    sign-with-keychain \
    send

# Check deployment status
if [ $? -eq 0 ]; then
    echo "Contract deployed successfully to $CONTRACT_ID"
else
    echo "Contract deployment failed"
    exit 1
fi
