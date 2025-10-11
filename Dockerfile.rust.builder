# --- Builder ---
FROM rust:1.85-slim-bookworm AS builder

ARG SOLANA_VERSION
ARG ANCHOR_VERSION
ARG PROGRAM_KEYPAIR_PATH

ENV DEBIAN_FRONTEND=noninteractive
ENV PATH="/root/.local/share/solana/install/active_release/bin:/root/.cargo/bin:/usr/local/bin:${PATH}"
ENV PROGRAM_KEYPAIR_PATH=${PROGRAM_KEYPAIR_PATH}

# Dependencies (minimal set for anchor + solana)
RUN apt-get update && apt-get install -y \
    build-essential pkg-config libssl-dev git python3 libudev-dev \
    ca-certificates wget gnupg bzip2 xz-utils curl jq protobuf-compiler libprotobuf-dev && \
    update-ca-certificates && rm -rf /var/lib/apt/lists/*

# Solana CLI (via Anza installer)
RUN curl -sSfL https://release.anza.xyz/stable/install | sh -s - v${SOLANA_VERSION}

# Anchor CLI
RUN cargo install anchor-cli@${ANCHOR_VERSION} --locked

WORKDIR /project

# Prepare Cargo deps. This is to cache dependencies.
COPY ./*.toml ./
COPY w3b2-solana-program/*.toml ./w3b2-solana-program/
COPY w3b2-solana-connector/*.toml ./w3b2-solana-connector/
COPY w3b2-solana-gateway/*.toml ./w3b2-solana-gateway/
COPY w3b2-solana-logger/*.toml ./w3b2-solana-logger/


RUN mkdir -p w3b2-solana-program/src w3b2-solana-connector/src w3b2-solana-gateway/src w3b2-solana-logger/src && \
    touch w3b2-solana-program/src/lib.rs && \
    touch w3b2-solana-connector/src/lib.rs && \
    touch w3b2-solana-logger/src/lib.rs && \
    echo "fn main() {}" > w3b2-solana-gateway/src/main.rs && \
    # Build an empty binary to cache dependencies for the whole workspace
    cargo build --workspace --release --bin w3b2-solana-gateway && \
    rm -rf target/release/w3b2-solana-gateway*

# Copy the application code
COPY . .

# Copy the build script
COPY build_and_deploy.sh /usr/local/bin/build_and_deploy.sh
RUN chmod +x /usr/local/bin/build_and_deploy.sh