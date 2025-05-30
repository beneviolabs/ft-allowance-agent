#!/bin/bash
set -e  # Exit on any error

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}Running Auth Proxy Contract Tests...${NC}"
cargo test -- --nocapture
echo ""

echo -e "${GREEN}Running Factory Contract Tests...${NC}"
cd factory
cargo test -- --nocapture
echo ""

echo -e "${GREEN}Running Integration Tests...${NC}"
cd ..
cargo test integration_tests -- --nocapture

# Check if any tests failed
if [ $? -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
