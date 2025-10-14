.PHONY: docker-build docker-run docker-test docker-clean help

# Docker commands for faster CI actions
docker-build:
	docker build -t near-contract-builder .

docker-run:
	docker run --rm -it -v $(PWD):/workspace -w /workspace near-contract-builder bash

docker-test:
	docker run --rm -v $(PWD):/workspace -w /workspace near-contract-builder bash -c "cd contracts && ./test.sh"

docker-build-contracts:
	docker run --rm -v $(PWD):/workspace -w /workspace near-contract-builder bash -c "cd contracts && ./build_auth_proxy.sh && cd factory && RUSTFLAGS='-Z unstable-options' cargo +nightly near build non-reproducible-wasm --no-abi --no-wasmopt"

docker-clean:
	docker system prune -f
	docker volume prune -f

# Local Build & Test with Docker
local-test:
	cd contracts && ./test.sh

local-build:
	cd contracts && ./build_auth_proxy.sh
	cd contracts/factory && RUSTFLAGS="-Z unstable-options" cargo +nightly near build non-reproducible-wasm --no-abi

help:
	@echo "Available commands:"
	@echo "  docker-build          - Build the Docker image"
	@echo "  docker-run            - Run interactive Docker container"
	@echo "  docker-test           - Run tests in Docker"
	@echo "  docker-build-contracts - Build contracts in Docker"
	@echo "  docker-clean          - Clean Docker system and volumes"
	@echo "  local-test            - Run tests locally (requires Rust)"
	@echo "  local-build           - Build contracts locally (requires Rust)"
