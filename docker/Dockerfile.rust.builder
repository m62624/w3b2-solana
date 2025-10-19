# --- Builder ---
FROM rust:1.90-slim-bookworm AS builder

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

WORKDIR /app

# Copy the application code first
COPY . .

# Copy utility scripts from the new scripts/ directory
COPY scripts/build_and_deploy.sh /usr/local/bin/
COPY scripts/run_gateway.sh /usr/local/bin/
COPY scripts/run_tests.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/build_and_deploy.sh /usr/local/bin/run_gateway.sh /usr/local/bin/run_tests.sh