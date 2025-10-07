# --- Builder ---
FROM rust:1.80-slim-bookworm AS builder

ARG SOLANA_VERSION=2.1.0
ARG ANCHOR_VERSION=0.31.1
ARG USER_ID=1000
ARG GROUP_ID=1000

ENV DEBIAN_FRONTEND=noninteractive
ENV PATH="/home/builder/.local/share/solana/install/active_release/bin:/home/builder/.cargo/bin:/usr/local/bin:${PATH}"

# deps (minimal set for anchor + solana)
RUN apt-get update && apt-get install -y \
    build-essential pkg-config libssl-dev git python3 python3-toml \
    libudev-dev ca-certificates wget gnupg bzip2 xz-utils sudo \
    protobuf-compiler libprotobuf-dev curl jq && \
    update-ca-certificates && rm -rf /var/lib/apt/lists/*

# Create a user to avoid running as root
RUN groupadd --gid $GROUP_ID builder && \
    useradd --uid $USER_ID --gid $GROUP_ID -m builder && \
    # Give the builder user passwordless sudo privileges
    echo "builder ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers


# Switch to the builder user BEFORE installing tools
USER builder
WORKDIR /home/builder

# Solana CLI (через Anza installer)
RUN curl -sSfL https://release.anza.xyz/stable/install | sh -s - v${SOLANA_VERSION}

# Anchor CLI
RUN cargo install anchor-cli@${ANCHOR_VERSION} --locked

WORKDIR /project

# Подготовка Cargo deps. This is to cache dependencies.
COPY --chown=builder:builder Cargo.toml Cargo.lock Anchor.toml ./
COPY --chown=builder:builder w3b2-solana-program/Cargo.toml ./w3b2-solana-program/
COPY --chown=builder:builder w3b2-solana-program/Xargo.toml ./w3b2-solana-program/
COPY --chown=builder:builder w3b2-solana-connector/Cargo.toml ./w3b2-solana-connector/
COPY --chown=builder:builder w3b2-solana-gateway/Cargo.toml ./w3b2-solana-gateway/

# Create dummy source files to allow `cargo fetch` to work
RUN mkdir -p w3b2-solana-program/src w3b2-solana-connector/src w3b2-solana-gateway/src && \
    touch w3b2-solana-program/src/lib.rs && \
    touch w3b2-solana-connector/src/lib.rs && \
    touch w3b2-solana-gateway/src/main.rs && \
    cargo fetch

# Copy the application code
COPY --chown=builder:builder . .

# Copy the build script
COPY --chown=builder:builder build_and_deploy.sh /usr/local/bin/build_and_deploy.sh
RUN chmod +x /usr/local/bin/build_and_deploy.sh