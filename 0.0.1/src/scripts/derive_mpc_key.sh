#!/bin/bash

# Configuration
NETWORK="testnet"
MPC_CONTRACT_ID="v1.signer-prod.testnet"

# Check for AGENT_PROXY_ACCOUNT if no argument provided
if [ -z "$1" ]; then
    if [ -z "$AGENT_PROXY_ACCOUNT" ]; then
        echo "Error: No account ID provided and AGENT_PROXY_ACCOUNT is not set"
        echo "Usage: ./derive_mpc_key.sh [account_id]"
        echo "Example: ./derive_mpc_key.sh example.testnet"
        exit 1
    fi
    ACCOUNT_ID="$AGENT_PROXY_ACCOUNT"
    echo "Using AGENT_PROXY_ACCOUNT: $ACCOUNT_ID"
else
    ACCOUNT_ID="$1"
    echo "Using provided account ID: $ACCOUNT_ID"
fi

# Get an account's keys and extract public key from full access line
PUBLIC_KEY=$(near list-keys "$ACCOUNT_ID" | grep "full access" | grep -o 'ed25519:[A-Za-z0-9]*')

if [ -z "$PUBLIC_KEY" ]; then
    echo "No full access ED25519 public key found for $ACCOUNT_ID"
    exit 1
fi

# make a near view call to the MPC key with the public key as the derivation path
echo "$PUBLIC_KEY"
DERIVED_KEY=$(near view $MPC_CONTRACT_ID derived_public_key "{\"path\":\"$PUBLIC_KEY\", \"predecessor\": \"$ACCOUNT_ID\"}" | tr -d '"')

echo "Derived MPC key: $DERIVED_KEY"

# Add the derived key as full access
echo "Adding derived key as full access key to $ACCOUNT_ID..."
echo "near account add-key "$ACCOUNT_ID" \
    grant-full-access \
    use-manually-provided-public-key $DERIVED_KEY \
    network-config $NETWORK \
    sign-with-keychain \
    send"

# Check result
if [ $? -eq 0 ]; then
    echo "Successfully added derived key to $ACCOUNT_ID"
else
    echo "Failed to add derived key"
    exit 1
fi

