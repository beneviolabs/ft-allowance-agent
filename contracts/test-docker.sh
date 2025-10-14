#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}Running Auth Proxy Contract Tests (Docker)...${NC}"
# Run unit tests only, skip integration tests that require sandbox
cargo test --lib --bins -- --nocapture
echo ""

echo -e "${GREEN}Running Factory Contract Tests (Docker)...${NC}"
cd factory
cargo test --lib -- --nocapture
echo ""

echo -e "${GREEN}Running Integration Tests (Docker - skipping sandbox tests)...${NC}"
cd ..
# Skip integration tests that require near-workspaces sandbox
cargo test integration_tests --lib -- --nocapture --skip sandbox
echo ""

# Check if any tests failed
if [ $? -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
