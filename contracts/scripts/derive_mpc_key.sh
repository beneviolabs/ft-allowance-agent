#!/bin/bash

# Configuration
if [ "$NEAR_ENV" = "mainnet" ]; then
    MPC_CONTRACT_ID="v1.signer"
else
    MPC_CONTRACT_ID="v1.signer-prod.testnet"
fi

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

# Save to environment variable
if [ -n "$ZSH_VERSION" ]; then
    PROFILE_FILE="$HOME/.zshrc"
elif [ -n "$BASH_VERSION" ]; then
    PROFILE_FILE="$HOME/.bash_profile"
else
    PROFILE_FILE="$HOME/.profile"
fi

# Remove any existing entries
sed -i '' '/export MPC_DERIVED_PK=/d' "$PROFILE_FILE"
sed -i '' '/export USER_PUBLIC_KEY_FOR_MPC=/d' "$PROFILE_FILE"

# Add new environment variables
echo "export USER_PUBLIC_KEY_FOR_MPC=\"$PUBLIC_KEY\"" >> "$PROFILE_FILE"
echo "export MPC_DERIVED_PK=\"$DERIVED_KEY\"" >> "$PROFILE_FILE"

# Export for current session
export MPC_DERIVED_PK="$DERIVED_KEY"
export USER_PUBLIC_KEY_FOR_MPC="$PUBLIC_KEY"

echo "Derived MPC key: $MPC_DERIVED_PK"
echo "MPC_DERIVED_PK has been added to $PROFILE_FILE"
echo "USER_PUBLIC_KEY_FOR_MPC has been added to $PROFILE_FILE"
echo "To apply changes in new terminals, please run: source $PROFILE_FILE"

# Add the derived key as full access
echo "Adding derived key as full access key to $ACCOUNT_ID..."
near account add-key "$ACCOUNT_ID" \
    grant-full-access \
    use-manually-provided-public-key $DERIVED_KEY \
    network-config $NEAR_ENV \
    sign-with-keychain \
    send

# Check result
if [ $? -eq 0 ]; then
    echo "Successfully added derived key to $ACCOUNT_ID"
else
    echo "Failed to add derived key"
    exit 1
fi

