# ---- Builder Stage ----
FROM rust:1.70 as builder

# Create a new empty shell project
WORKDIR /usr/src
RUN USER=root cargo new --bin rust-chat-simulation
WORKDIR /usr/src/rust-chat-simulation

# Copy our manifests
COPY Cargo.toml Cargo.lock ./

# Copy the source code
COPY src ./src

# Build for release
RUN cargo build --release

# ---- Final Stage ----
FROM debian:buster-slim

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/rust-chat-simulation/target/release/rust-chat-simulation /usr/local/bin/rust-chat-simulation

# Set the binary as the entrypoint
CMD ["rust-chat-simulation"]