#!/bin/bash

# Check if account to authorize is provided
if [ -z "$1" ]; then
    echo "Error: Account to authorize is required"
    echo "Usage: ./add_authorized_agent.sh <account_id>"
    echo "Example: ./add_authorized_agent.sh benevio-labs.testnet"
    exit 1
fi

ACCOUNT_TO_AUTHORIZE="$1"

# Check if AGENT_PROXY_ACCOUNT is set
if [ -z "$AGENT_PROXY_ACCOUNT" ]; then
    echo "Error: AGENT_PROXY_ACCOUNT environment variable is not set"
    exit 1
fi

# Check if AGENT_PARENT_ACCOUNT is set
if [ -z "$AGENT_PARENT_ACCOUNT" ]; then
    echo "Error: AGENT_PARENT_ACCOUNT environment variable is not set"
    exit 1
fi

echo "Adding authorized user $ACCOUNT_TO_AUTHORIZE to contract $AGENT_PROXY_ACCOUNT..."

# Execute the contract call
near contract call-function as-transaction \
    "$AGENT_PROXY_ACCOUNT" \
    add_authorized_user \
    json-args '{"account_id": "'"$ACCOUNT_TO_AUTHORIZE"'"}' \
    prepaid-gas '100.0 Tgas' \
    attached-deposit '0 NEAR' \
    sign-as "$AGENT_PARENT_ACCOUNT" \
    network-config $NEAR_ENV \
    sign-with-keychain \
    send

# Check result
if [ $? -eq 0 ]; then
    echo "Successfully added $ACCOUNT_TO_AUTHORIZE as authorized user"
else
    echo "Failed to add authorized user"
    exit 1
fi
