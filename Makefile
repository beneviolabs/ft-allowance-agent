.PHONY: docker-build docker-run docker-test docker-clean help

# Build  contracts - NOTICE! This produces a different wasm/hash than ./build_auth_proxy.sh due to the --no-wasmopt flag which is required to avoid an incompatibility issue withthe global memory feature in docker/cargo
BUILD_CMD = RUSTFLAGS='-Z unstable-options' cargo +nightly near build non-reproducible-wasm --no-abi --no-wasmopt

# Docker commands for faster CI actions
docker-build:
	docker build -t near-contract-builder .

docker-test:
	docker run --rm -v $(PWD):/workspace -w /workspace near-contract-builder bash -c "cd contracts && ./test-docker.sh"

# Build contracts using consistent build command
docker-build-contracts:
	docker run --rm -v $(PWD):/workspace -w /workspace near-contract-builder bash -c "cd contracts && $(BUILD_CMD) && cd factory && $(BUILD_CMD)"

docker-clean:
	docker system prune -f
	docker volume prune -f

# Local Debugging of Docker container
local-docker-run:
	docker run --rm -it -v $(PWD):/workspace -w /workspace near-contract-builder bash


help:
	@echo "Available commands:"
	@echo "  docker-build          - Build the Docker image"
	@echo "  docker-test           - Run tests in Docker"
	@echo "  docker-build-contracts - Build contracts in Docker"
	@echo "  docker-clean          - Clean Docker system and volumes"
	@echo "  local-docker-run      - Run interactive Docker container"
