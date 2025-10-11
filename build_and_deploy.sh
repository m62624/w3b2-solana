#!/bin/bash
set -e

# --- Configuration ---
# These variables are expected to be set in the environment (e.g., from .env file).
DEPLOYER_KEYPAIR_PATH="${DEPLOYER_KEYPAIR_PATH:-/keys/deployer-keypair.json}"
PROGRAM_KEYPAIR_PATH="${PROGRAM_KEYPAIR_PATH:?Error: PROGRAM_KEYPAIR_PATH is not set.}"
SOLANA_RPC_URL="${SOLANA_RPC_URL:-http://localhost:8899}"

PROGRAM_DIR="w3b2-solana-program"
LIB_RS_PATH="$PROGRAM_DIR/src/lib.rs"
ANCHOR_TOML_PATH="Anchor.toml"
IDL_PATH_TEMPLATE="target/idl/w3b2_solana_program.json"
DEPLOY_SO_PATH="target/deploy/w3b2_solana_program.so"
ARTIFACTS_DIR="artifacts"

# --- Helper Functions ---
print_help() {
    echo "Usage: $0 [MODE]"
    echo "Modes:"
    echo "  --build-only    Build the Anchor program and update IDL (default)."
    echo "  --deploy        Build and deploy the program to a validator."
    echo "  --help          Show this help message."
}

# --- Main Logic ---

# 1. Parse command-line arguments
MODE="--build-only"
if [ "$1" ]; then
    MODE=$1
fi

if [[ "$MODE" == "--help" ]]; then
    print_help
    exit 0
fi

# --- Main Logic ---
if [[ "$MODE" == "--deploy" ]]; then
    # --- DEPLOY LOGIC ---
    # The deployer only needs the keypair to get the program ID.
    # It assumes artifacts already exist from the builder service.
    if [ ! -f "$PROGRAM_KEYPAIR_PATH" ]; then
        echo "❌ Error: Program keypair not found at $PROGRAM_KEYPAIR_PATH for deployment."
        exit 1
    fi

    if [ ! -f "$DEPLOYER_KEYPAIR_PATH" ]; then
        echo "Deployer keypair not found at $DEPLOYER_KEYPAIR_PATH. Creating a new one..."
        mkdir -p "$(dirname "$DEPLOYER_KEYPAIR_PATH")"
        solana-keygen new --no-passphrase -o "$DEPLOYER_KEYPAIR_PATH"
        echo "New deployer keypair created."
    fi
    export PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_KEYPAIR_PATH")
    echo "🔑 Program ID for deployment: $PROGRAM_ID"

    echo "💰 Requesting airdrop for deployer wallet..."
    # Airdrop some SOL to the deployer wallet to pay for fees.
    # We add a small retry loop in case the validator is not ready yet.
    until solana airdrop 5 "$DEPLOYER_KEYPAIR_PATH" --url "$SOLANA_RPC_URL"; do
      echo "Airdrop failed, retrying in 2 seconds..."
      sleep 2
    done
    echo "✅ Airdrop successful."

    echo "🚀 Deploying program to $SOLANA_RPC_URL..."
    solana program deploy "$ARTIFACTS_DIR/w3b2_solana_program.so" \
        --program-id "$PROGRAM_KEYPAIR_PATH" \
        --url "$SOLANA_RPC_URL" \
        --keypair "$DEPLOYER_KEYPAIR_PATH" # Explicitly specify the fee payer
    echo "✅ Program deployed successfully."

elif [[ "$MODE" == "--build-only" ]]; then
    # --- BUILD LOGIC ---

    # Create artifacts directory if it doesn't exist
    mkdir -p "$ARTIFACTS_DIR"

    if [ ! -f "$PROGRAM_KEYPAIR_PATH" ]; then
        echo "Program keypair not found at $PROGRAM_KEYPAIR_PATH. Creating a new one..."
        mkdir -p "$(dirname "$PROGRAM_KEYPAIR_PATH")"
        solana-keygen new --no-passphrase -o "$PROGRAM_KEYPAIR_PATH"
        echo "New keypair created."
    fi

    export PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_KEYPAIR_PATH")
    echo "🔑 Program ID: $PROGRAM_ID"

    echo " Patching source files with Program ID..."
    sed -i -E 's/(declare_id!\s*\(\s*").*("\)\s*;)/\1'"$PROGRAM_ID"'\2/' "$LIB_RS_PATH"
    sed -i -E 's/(w3b2_solana_program\s*=\s*\").*(\")/\1'"$PROGRAM_ID"'\2/' "$ANCHOR_TOML_PATH"
    echo "✅ Source files patched."

    echo "🚀 Building Anchor workspace..."
    anchor build
    echo "🚀 Building gateway binary..."
    cargo build --release --bin w3b2-solana-gateway
    echo "✅ Builds successful."

    echo "🔄 Finalizing artifacts..."
    jq ".metadata.address = \"$PROGRAM_ID\"" "$IDL_PATH_TEMPLATE" > "$ARTIFACTS_DIR/w3b2_solana_program.json"
    cp "$DEPLOY_SO_PATH" "$ARTIFACTS_DIR/"

    echo "✅ Artifacts created in $ARTIFACTS_DIR/"
    echo "✅ Build complete."

    # --- Final Permission Fix ---
    # After all files are created by the root user inside the container,
    # change their ownership to the host user's UID/GID.
    # HOST_UID and HOST_GID are passed from the docker-compose command.
    if [ -n "$HOST_UID" ] && [ -n "$HOST_GID" ]; then
        echo "Changing ownership of generated files to $HOST_UID:$HOST_GID..."
        chown -R "$HOST_UID:$HOST_GID" "$ARTIFACTS_DIR" "target" "$(dirname "$PROGRAM_KEYPAIR_PATH")"
    fi
fi

echo "Program ID: $PROGRAM_ID"

exit 0
