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

# Fetch the current nonce from the mpc public key
NONCE=$(curl -s -X POST https://rpc.testnet.near.org \
    -H 'Content-Type: application/json' \
    -d '{
        "jsonrpc": "2.0",
        "id": "dontcare",
        "method": "query",
        "params": {
            "request_type": "view_access_key",
            "finality": "final",
            "account_id": "'"$AGENT_PROXY_ACCOUNT"'",
            "public_key": "'"$MPC_DERIVED_PK"'"
        }
    }' | grep -o '"nonce":[0-9]*' | grep -o '[0-9]*')

if [ -z "$NONCE" ]; then
    echo "Failed to extract current nonce from RPC response"
    exit -1
fi

# Increase nonce by 10
NONCE=$((NONCE + 10))
echo "Using nonce: $NONCE"
# Transaction parameters
ARGS='{
    "contract_id": "wrap.testnet",
    "method_name": "near_deposit",
    "args": [],
    "gas": "300000000000000",
    "deposit": "1000000000000000000000000",
    "nonce": "'"$NONCE"'",
    "block_hash": "'"$BLOCK_HASH"'",
    "mpc_signer_pk": "'"$MPC_DERIVED_PK"'"
}'

# Make the contract call
near call $AGENT_PROXY_ACCOUNT request_signature "$ARGS" \
    --accountId $AGENT_ID \
    --deposit 1 \
    --gas 50000000000000
