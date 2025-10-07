# --- Builder ---
FROM rust:1.80-slim-bookworm AS builder

ARG SOLANA_VERSION=2.1.0
ARG ANCHOR_VERSION=0.31.1
ARG USER_ID=1000
ARG GROUP_ID=1000

ENV DEBIAN_FRONTEND=noninteractive
ENV PATH="/home/builder/.cargo/bin:/usr/local/bin:${PATH}"

# deps (минимальный набор для anchor + solana)
RUN apt-get update && apt-get install -y \
    build-essential pkg-config libssl-dev git python3 python3-toml \
    libudev-dev ca-certificates wget gnupg bzip2 xz-utils \
    protobuf-compiler libprotobuf-dev curl jq && \
    update-ca-certificates && rm -rf /var/lib/apt/lists/*

# Создаем пользователя, чтобы не работать под root
RUN groupadd --gid $GROUP_ID builder && \
    useradd --uid $USER_ID --gid $GROUP_ID -m builder
ENV USER=builder

# Solana CLI (через Anza installer)
RUN curl -sSfL https://release.anza.xyz/stable/install | sh -s - v${SOLANA_VERSION}
ENV PATH="/home/builder/.local/share/solana/install/active_release/bin:${PATH}"

# Anchor CLI
RUN cargo install anchor-cli@${ANCHOR_VERSION} --locked --force

WORKDIR /project

# Подготовка Cargo deps. This is to cache dependencies.
COPY Cargo.toml ./
COPY w3b2-solana-program/Cargo.toml ./w3b2-solana-program/
COPY w3b2-solana-connector/Cargo.toml ./w3b2-solana-connector/
COPY w3b2-solana-gateway/Cargo.toml ./w3b2-solana-gateway/
RUN mkdir -p w3b2-solana-program/src w3b2-solana-connector/src w3b2-solana-gateway/src && \
    touch w3b2-solana-program/src/lib.rs && \
    touch w3b2-solana-connector/src/main.rs && \
    touch w3b2-solana-gateway/src/main.rs && \
    cargo fetch

# Копируем код
COPY . .

# Меняем владельца, чтобы cargo build не ругался
RUN chown -R builder:builder /project

# Копируем скрипт сборки
COPY build_and_deploy.sh /usr/local/bin/build_and_deploy.sh
RUN chmod +x /usr/local/bin/build_and_deploy.sh

USER builder