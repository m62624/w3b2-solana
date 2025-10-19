#!/bin/bash
set -e

# --- Configuration ---
# These variables are expected to be set in the environment (e.g., from .env file).
DEPLOYER_KEYPAIR_PATH="${DEPLOYER_KEYPAIR_PATH:-/keys/deployer-keypair.json}"
PROGRAM_KEYPAIR_PATH="${PROGRAM_KEYPAIR_PATH:?Error: PROGRAM_KEYPAIR_PATH is not set.}"
SOLANA_RPC_URL="${SOLANA_RPC_URL:-http://localhost:8899}"
PROGRAM_NAME="${PROGRAM_NAME:-w3b2_solana_program}"
PROGRAM_DIR_NAME="${PROGRAM_DIR_NAME:-w3b2-solana-program}"
PROGRAM_IDL_FILENAME="${PROGRAM_IDL_FILENAME:-w3b2_solana_program.idl.json}"
PROGRAM_SO_FILENAME="${PROGRAM_SO_FILENAME:-w3b2_solana_program.so}"

LIB_RS_PATH="$PWD/$PROGRAM_DIR_NAME/src/lib.rs"
ANCHOR_TOML_PATH="$PWD/Anchor.toml"
IDL_SOURCE_PATH="$PWD/target/idl/${PROGRAM_NAME}.json" # Anchor always generates a .json file without suffix
DEPLOY_SO_PATH="$PWD/target/deploy/${PROGRAM_NAME}.so"
ARTIFACTS_DIR="$PWD/artifacts"

# --- Cleanup and Permissions ---
fix_permissions() {
    # Default to user 1000 if HOST_UID/GID are not set. This is the standard first user in most Linux distros.
    local uid=${HOST_UID:-1000}
    local gid=${HOST_GID:-1000}

    echo "Fixing permissions for generated files to $uid:$gid..."
    # Use chown on directories that are known to be created or modified by the container.
    # Adding `|| true` prevents the script from failing if a directory doesn't exist yet.
    chown -R "$uid:$gid" "$ARTIFACTS_DIR" "$(dirname "$PROGRAM_KEYPAIR_PATH")" || true
    if [ -d "target" ]; then
        chown -R "$uid:$gid" "target" || true
    fi
}

# Set a trap to run the permission fix function on script exit (normal or error).
trap fix_permissions EXIT

# --- Helper Functions ---
print_help() {
    echo "Usage: $0 [MODE]"
    echo "Modes:"
    echo "  --build-only    Build the Anchor program and gateway binary (default)."
    echo "  --deploy        Deploy the pre-built program to a validator."
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
    solana program deploy "$ARTIFACTS_DIR/$PROGRAM_SO_FILENAME" \
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
    sed -i -E 's/('"$PROGRAM_NAME"'\s*=\s*\").*(\")/\1'"$PROGRAM_ID"'\2/' "$ANCHOR_TOML_PATH"
    echo "✅ Source files patched."

    echo "🚀 Building Anchor workspace...BUILD IN PROGRESS — PLEASE WAIT FOR NEW LOGS FROM builder-1.
This may take some time. Other containers such as solana-validator-1 and docs-1 may already be running."
    anchor build
    echo "🚀 Building gateway binary..."
    cargo build --release --bin w3b2-solana-gateway
    echo "✅ Builds successful."

    echo "🔄 Finalizing artifacts..."
    jq ".metadata.address = \"$PROGRAM_ID\"" "$IDL_SOURCE_PATH" > "$ARTIFACTS_DIR/$PROGRAM_IDL_FILENAME"
    cp "$DEPLOY_SO_PATH" "$ARTIFACTS_DIR/"

    echo "✅ Artifacts created in $ARTIFACTS_DIR/"
    echo "✅ Build complete."

fi

echo "Program ID: $PROGRAM_ID"

exit 0