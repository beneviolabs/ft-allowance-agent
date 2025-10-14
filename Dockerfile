# Multi-stage Dockerfile for NEAR contract development
ARG PLATFORM=linux/amd64
FROM --platform=$PLATFORM ubuntu:22.04

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    git \
    libudev-dev \
    pkg-config \
    binaryen \
    build-essential \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Rust toolchains and targets
RUN rustup toolchain install stable nightly
RUN rustup target add wasm32-unknown-unknown --toolchain stable
RUN rustup target add wasm32-unknown-unknown --toolchain nightly
RUN rustup component add rustfmt clippy --toolchain stable
RUN rustup component add rust-src --toolchain nightly

# Install cargo-near
RUN cargo install cargo-near --version 0.15.0 --locked

# Set working directory
WORKDIR /workspace

# Copy Cargo files for dependency caching
COPY contracts/Cargo.toml contracts/Cargo.lock ./
COPY contracts/factory/Cargo.toml contracts/factory/Cargo.lock ./factory/

# Pre-compile dependencies (this layer will be cached)
# Create minimal source files that match the expected structure
RUN echo "// Dummy auth_proxy module for dependency caching" > auth_proxy.rs && \
    echo "// Dummy actions module" > actions.rs && \
    echo "// Dummy models module" > models.rs && \
    echo "// Dummy serializer module" > serializer.rs && \
    echo "// Dummy utils module" > utils.rs && \
    echo "// Dummy integration_tests module" > integration_tests.rs && \
    echo "// Dummy unit_tests module" > unit_tests.rs

RUN cargo build --release

# Create dummy factory file and build
RUN cd factory && \
    echo "// Dummy factory module for dependency caching" > factory.rs && \
    echo "// Dummy unit_tests module" > unit_tests.rs && \
    cargo build --release

# Remove dummy source files
RUN rm -f auth_proxy.rs actions.rs models.rs serializer.rs utils.rs integration_tests.rs unit_tests.rs && \
    rm -rf target && \
    cd factory && rm -f factory.rs unit_tests.rs && rm -rf target

# Copy the rest of the source code
COPY contracts/ ./

# Default command
CMD ["bash"]
