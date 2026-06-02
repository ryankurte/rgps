FROM debian:bookworm-slim

# Install the build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    git \
    libudev-dev \
    libssl-dev \
    musl-dev \
    openssl \
    pkg-config \
    ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Zig
RUN mkdir -p /root/.zig
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://ziglang.org/builds/zig-x86_64-linux-0.17.0-dev.644+3de725074.tar.xz | tar -xJC /root/.zig --strip-components=1
ENV PATH="/root/.zig:${PATH}"

# Install cargo binstall
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# Install cargo zigbuild
RUN cargo binstall cargo-zigbuild

# Add rust targets
RUN rustup target add aarch64-unknown-linux-musl armv7-unknown-linux-musleabihf
