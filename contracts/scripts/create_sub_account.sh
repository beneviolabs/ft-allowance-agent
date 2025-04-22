#!/bin/bash

# Configuration
INITIAL_BALANCE="1"

# Check if parent account is provided
if [ -z "$1" ]; then
    echo "Error: Parent account is required"
    echo "Usage: ./create_sub_account.sh <parent_account>"
    echo "Example: ./create_sub_account.sh youraccount.testnet"
    exit 1
fi

PARENT_ACCOUNT="$1"

# Validate parent account format
if [[ ! $PARENT_ACCOUNT =~ ^[a-z0-9_-]+\.(testnet|near)$ ]]; then
    echo "Error: Invalid parent account format"
    echo "Account must end with .testnet or .near and contain only lowercase letters, numbers, hyphens, and underscores"
    exit 1
fi

# Generate agent account name from parent account
NEW_ACCOUNT="agent.$PARENT_ACCOUNT"
echo "Creating agent account: $NEW_ACCOUNT"

# Create and fund the account
echo "Creating account $NEW_ACCOUNT with $INITIAL_BALANCE NEAR..."
near account create-account fund-myself \
    $NEW_ACCOUNT \
    "$INITIAL_BALANCE NEAR" \
    autogenerate-new-keypair \
    save-to-keychain \
    sign-as $PARENT_ACCOUNT \
    network-config $NEAR_ENV \
    sign-with-keychain \
    send

# Fund the sub-account with additional NEAR
echo "Funding $NEW_ACCOUNT with 5 NEAR..."
near send $PARENT_ACCOUNT $NEW_ACCOUNT 5

# Check the result
if [ $? -eq 0 ]; then
    echo "Successfully created account $NEW_ACCOUNT"
    echo "To view the account details:"
    echo "near state $AGENT_PROXY_ACCOUNT"
else
    echo "Failed to create account"
    exit 1
fi

# Check shell type and set appropriate profile file
if [ -n "$ZSH_VERSION" ]; then
    PROFILE_FILE="$HOME/.zshrc"
elif [ -n "$BASH_VERSION" ]; then
    PROFILE_FILE="$HOME/.bash_profile"
else
    PROFILE_FILE="$HOME/.profile"
fi

# Remove any existing entries
sed -i '' '/export AGENT_PROXY_ACCOUNT=/d' "$PROFILE_FILE"
sed -i '' '/export AGENT_PARENT_ACCOUNT=/d' "$PROFILE_FILE"

# Add new environment variables
cat << EOF >> "$PROFILE_FILE"
export AGENT_PROXY_ACCOUNT="$NEW_ACCOUNT"
export AGENT_PARENT_ACCOUNT="$PARENT_ACCOUNT"
EOF

# Export variables for current session
export AGENT_PROXY_ACCOUNT="$NEW_ACCOUNT"
export AGENT_PARENT_ACCOUNT="$PARENT_ACCOUNT"

# Print confirmation
echo "Environment variables have been set:"
echo "AGENT_PROXY_ACCOUNT=$AGENT_PROXY_ACCOUNT"
echo "AGENT_PARENT_ACCOUNT=$AGENT_PARENT_ACCOUNT"
echo "Variables have been added to $PROFILE_FILE"
echo "To apply changes in new terminals, please run: source $PROFILE_FILE"
