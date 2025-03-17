#!/bin/bash

# Check if block hash is provided
if [ -z "$1" ]; then
    echo "Error: No block hash provided"
    echo "Usage: ./request_signature.sh <block_hash> [agent_id]"
    echo "Example: ./request_signature.sh abc123... autonomous-agent.testnet"
    exit 1
fi

# Set agent ID from parameter or use default
AGENT_ID="${2:-benevio-labs.testnet}"
echo "Using agent ID: $AGENT_ID"

# Store block hash from parameter
BLOCK_HASH="$1"

# Calculate nonce from current timestamp in milliseconds
NONCE=$(date +%s | cut -b1-13)

# Transaction parameters
ARGS='{
    "contract_id": "wrap.testnet",
    "method_name": "near_deposit",
    "args": [],
    "gas": "0",
    "deposit": "1000000000000000000000000",
    "nonce": "'"$NONCE"'",
    "block_hash": "'"$BLOCK_HASH"'"
}'

# Make the contract call
near call $AGENT_PROXY_ACCOUNT request_signature "$ARGS" \
    --accountId $AGENT_ID \
    --deposit 1
