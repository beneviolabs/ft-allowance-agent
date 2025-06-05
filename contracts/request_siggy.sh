#!/bin/bash

# Check if block hash and command are provided
if [ -z "$1" ] || [ -z "$2" ]; then
    echo "Error: Missing required parameters"
    echo "Usage: ./request_signature.sh <block_hash> <command> [agent_id]"
    echo "Commands: add_key, sign_message, deposit"
    echo "Example: ./request_signature.sh abc123... deposit autonomous-agent.testnet"
    exit 1
fi

# Store parameters
BLOCK_HASH="$1"
COMMAND="$2"
if [ -z "$3" ]; then
    if [ "$NEAR_ENV" = "mainnet" ]; then
        AGENT_ID="peerfolio.near"
    else
        AGENT_ID="peerfolio.testnet"
    fi
else
    AGENT_ID="$3"
fi
echo "Using agent ID: $AGENT_ID with command: $COMMAND"
echo "calling https://rpc.$NEAR_ENV.fastnear.com"

# Fetch the current nonce from the mpc public key
RESPONSE=$(curl -s -X POST https://rpc.$NEAR_ENV.fastnear.com \
    -H 'Content-Type: application/json' \
    -d '{
        "jsonrpc": "2.0",
        "id": "benevio.dev",
        "method": "query",
        "params": {
            "request_type": "view_access_key",
            "finality": "final",
            "account_id": "'"$AGENT_PROXY_ACCOUNT"'",
            "public_key": "'"$MPC_DERIVED_PK"'"
        }
    }')
NONCE=$( echo $RESPONSE | grep -o '"nonce":[0-9]*' | grep -o '[0-9]*')

if [ -z "$NONCE" ]; then
    echo "Failed to extract current nonce from RPC response: $RESPONSE"
    exit -1
fi

# Increase nonce by 10
NONCE=$((NONCE + 10))
echo "Using nonce: $NONCE"

execute_add_key() {
    echo "Executing add_public_key..."
    if [ "$NEAR_ENV" = "mainnet" ]; then
        CONTRACT_ID="intents.near"
    else
        echo "Error: add_key command is only supported on mainnet"
        exit 1
    fi
    ADD_PUBLIC_KEY_ARGS='{
        "contract_id": "'"$CONTRACT_ID"'",
        "method_name": "add_public_key",
        "args": "{\"public_key\":\"'$USER_PUBLIC_KEY_FOR_MPC'\"}",
        "gas": "300000000000000",
        "deposit": "1",
        "nonce": "'"$NONCE"'",
        "block_hash": "'"$BLOCK_HASH"'",
        "mpc_signer_pk": "'"$MPC_DERIVED_PK"'",
        "derivation_path": "'"$AGENT_PROXY_ACCOUNT"'"
    }'
    near call $AGENT_PROXY_ACCOUNT request_signature "$ADD_PUBLIC_KEY_ARGS" \
        --accountId $AGENT_ID \
        --deposit 0 \
        --gas 100000000000000
}

execute_deposit() {
    if [ "$NEAR_ENV" = "mainnet" ]; then
        CONTRACT_ID="wrap.near"
    else
        CONTRACT_ID="wrap.testnet"
    fi
    echo "Executing near_deposit..."
    DEPOSIT_ARGS='{
        "contract_id": "'"$CONTRACT_ID"'",
        "actions_json": "[{\"type\":\"FunctionCall\", \"deposit\": \"50000000000000000000000\", \"gas\": \"300000000000000\", \"method_name\": \"near_deposit\", \"args\": \"\"}]",
        "nonce": "'"$NONCE"'",
        "block_hash": "'"$BLOCK_HASH"'",
        "mpc_signer_pk": "'"$MPC_DERIVED_PK"'",
        "derivation_path": "'"$AGENT_PROXY_ACCOUNT"'"
    }'
    near call $AGENT_PROXY_ACCOUNT request_signature "$DEPOSIT_ARGS" \
        --accountId $AGENT_ID \
        --deposit 1 \
        --gas 100000000000000
}

# Execute command based on input
case "$COMMAND" in
    "add_key")
        execute_add_key
        ;;
    "deposit")
        execute_deposit
        ;;
    *)
        echo "Error: Invalid command. Use 'add_key' or 'deposit'"
        exit 1
        ;;
esac
