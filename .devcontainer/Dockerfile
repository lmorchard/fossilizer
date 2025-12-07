# Rust development container for Fossilizer
FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

# Install additional dependencies for building fossilizer
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
        musl-tools \
        musl-dev \
        pkg-config \
        libssl-dev \
        build-essential \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Add musl target for cross-compilation
RUN rustup target add x86_64-unknown-linux-musl

# Install useful Rust tools
RUN rustup component add clippy rustfmt
