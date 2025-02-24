#!/bin/bash

# Check if signed transaction is provided
if [ -z "$1" ]; then
    echo "Error: No signed transaction provided"
    echo "Usage: ./submit_tx.sh <signed_txn>"
    exit 1
fi

# Convert signed transaction to base64
SIGNED_TX_BASE64=$(echo -n "$1" | base64)

# Submit transaction to NEAR RPC
curl -X POST https://rpc.testnet.fastnear.com \
  -H 'Content-Type: application/json' \
  -d "{
    \"jsonrpc\": \"2.0\",
    \"id\": \"fastnear\",
    \"method\": \"send_tx\",
    \"params\": {
      \"signed_tx_base64\": \"$SIGNED_TX_BASE64\",
      \"wait_until\": \"EXECUTED\"
    }
  }"
