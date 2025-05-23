#!/bin/bash

# Check if NEAR CLI is installed
if ! command -v near &> /dev/null; then
    echo "NEAR CLI is not installed. Please install it first with: npm install -g near-cli"
    exit 1
fi

# Check required tools
check_requirements() {
    # Check if NEAR CLI is installed
    if ! command -v near &> /dev/null; then
        echo "NEAR CLI is not installed. Please install it first with: npm install -g near-cli"
        exit 1
    fi

    # Check if wasm-opt is installed
    if ! command -v wasm-opt &> /dev/null; then
        echo "Installing wasm-opt via Homebrew..."
        if ! command -v brew &> /dev/null; then
            echo "Homebrew not found. Please install from https://brew.sh"
            exit 1
        fi
        brew install binaryen
    fi

    # Check if wasm32 target is installed for nightly
    if ! rustup target list --installed --toolchain nightly | grep -q "wasm32-unknown-unknown"; then
        echo "Installing wasm32 target for nightly toolchain..."
        rustup target add wasm32-unknown-unknown --toolchain nightly
    fi

    # Check if wasm32 target is installed for stable
    if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
        echo "Installing wasm32 target for stable toolchain..."
        rustup target add wasm32-unknown-unknown
    fi
}

# Run requirement checks
check_requirements

# Clear previous builds
echo "Clearing previous builds..."
cargo clean

echo "Running cargo formatter "
cargo fmt

# Set appropriate network based on NEAR_ENV
if [ "$NEAR_ENV" = "testnet" ]; then
  NETWORK="testnet"
else
  NETWORK="near"
fi
# Build the contract
echo "Building contract..."
cd ../ && RUSTFLAGS="-Z unstable-options" cargo +nightly near build --no-docker --no-abi

# Set variables
WASM_PATH="target/near/proxy_contract.wasm"
FACTORY_ACCOUNT="proxy-v1.benevio-labs.$NETWORK"
FACTORY_OWNER="benevio-labs.$NETWORK"

echo "Optimizing WASM..."
wasm-opt -Oz -o "$WASM_PATH.optimized" "$WASM_PATH"
mv "$WASM_PATH.optimized" "$WASM_PATH"

# Verify WASM magic header after optimization
echo "Verifying WASM header..."
if ! xxd -p -l 4 "$WASM_PATH" | grep -q "0061736d"; then
    echo "❌ Invalid WASM header! Expected '0061736d' (\\0asm)"
    echo "First 4 bytes: $(xxd -p -l 4 "$WASM_PATH")"
    exit 1
else
    echo "✅ Valid WASM header verified"
fi

# Check if WASM file exists
if [ ! -f "$WASM_PATH" ]; then
    echo "Error: WASM file not found at $WASM_PATH"
    exit 1
fi

# Before making the near call, log the exact input
debug_chunk() {
    local chunk="$1"
    echo "Chunk length: ${#chunk}"
    echo "First 100 chars of chunk: ${chunk:0:100}"
    echo "JSON to be sent:"
    echo "{\"code\": \"$chunk\"}" | jq '.'
}

# Deploy factory if needed
if ! near state "$FACTORY_ACCOUNT" &>/dev/null; then
    echo "Deploying factory contract..."
    near create-account \
        "$FACTORY_ACCOUNT" \
        --masterAccount "$FACTORY_OWNER" \
        --initialBalance "6"

    near deploy \
    "$FACTORY_ACCOUNT" \
    "$WASM_PATH" \
    --initFunction "new" \
    --initArgs '{"owner_id":"'"$FACTORY_OWNER"'"}'
else

    # Update proxy code using chunked base64 input
    echo "Updating proxy code..."
    ENCODED_WASM=$(base64 < "$WASM_PATH")
    CHUNK_SIZE=100000
    total_size=$(echo -n "$ENCODED_WASM" | wc -c)
    processed_size=0

    # Split base64 encoded WASM into chunks and process each chunk
    update_success=true
    echo "$ENCODED_WASM" | fold -w $CHUNK_SIZE | while read -r chunk; do
        debug_chunk "$chunk"
        # Prepare the command but don't execute yet
        command="near call \"$FACTORY_ACCOUNT\" \
            update_proxy_code \
            "$chunk" --base64  \
            --accountId \"$FACTORY_OWNER\" \
            --gas 300000000000000"

        # Show the command that will be executed
        echo "About to execute:"
        echo "$command"

        # Wait for user confirmation
        read -p "Press enter to execute this command (or Ctrl+C to abort)..."

        # Execute the command
        if ! eval "$command"; then
            update_success=false
            break
        fi
    done

    # Check if update was successful
    if [ "$update_success" = false ]; then
        echo "Failed to update proxy code"
        exit 1
    fi
    echo "Successfully uploaded proxy code in chunks"
fi


# Generate and verify checksum format
echo "Generating WASM checksum..."
WASM_CHECKSUM=$(shasum -a 256 "$WASM_PATH" | cut -d ' ' -f 1)
echo "WASM checksum (hex): 0x$WASM_CHECKSUM"

echo "Waiting 2 seconds for block finality before checksum verification..."
sleep 2
# Verify length is correct for SHA-256 (64 hex characters)
if [ ${#WASM_CHECKSUM} -eq 64 ]; then
    echo "✓ Checksum verified (32 bytes/64 hex characters)"
else
    echo "✗ Invalid checksum length"
    exit 1
fi

# Get deployed contract code hash
echo "Fetching deployed contract hash..."
DEPLOYED_HASH=$(near state "$FACTORY_ACCOUNT" | grep "Contract (SHA-256 checksum hex)" | awk '{print $NF}')

if [ -z "$DEPLOYED_HASH" ]; then
    echo "❌ Failed to fetch deployed contract hash"
    exit 1
fi

if [ "$WASM_CHECKSUM" != "$DEPLOYED_HASH" ]; then
    echo "❌ Checksum mismatch!"
    echo "Local WASM:    0x$WASM_CHECKSUM"
    echo "Deployed code: 0x$DEPLOYED_HASH"
    exit 1
else
    echo "✅ Checksum match confirmed"
fi

# Check deployment status
if [ $? -eq 0 ]; then
    echo "Factory Contract updated successfully at $FACTORY_ACCOUNT"
else
    echo "Factory Contract deployment failed"
    exit 1
fi
