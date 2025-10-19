#!/bin/bash
set -e

# --- Configuration ---
PROGRAM_NAME="${PROGRAM_NAME:-w3b2_solana_program}"
PROGRAM_DIR_NAME="${PROGRAM_DIR_NAME:-w3b2-solana-program}"
PROGRAM_IDL_FILENAME="${PROGRAM_IDL_FILENAME:-w3b2_solana_program.idl.json}"
PROGRAM_SO_FILENAME="${PROGRAM_SO_FILENAME:-w3b2_solana_program.so}"

# --- Cleanup and Permissions ---
fix_permissions() {
    # Default to user 1000 if HOST_UID/GID are not set. This is the standard first user in most Linux distros.
    local uid=${HOST_UID:-1000}
    local gid=${HOST_GID:-1000}

    echo "Fixing permissions for generated files to $uid:$gid..."
    # The tester service primarily modifies the `target` directory.
    chown -R "$uid:$gid" "$PWD/target" || true
}

# Set a trap to run the permission fix function on script exit (normal or error).
trap fix_permissions EXIT

# Construct the validator URL from environment variables, with defaults.
SOLANA_RPC_HOST="${SOLANA_RPC_HOST:-solana-validator}"
SOLANA_VALIDATOR_RPC_PORT_INTERNAL="${SOLANA_VALIDATOR_RPC_PORT_INTERNAL:-8899}"
VALIDATOR_URL="http://${SOLANA_RPC_HOST}:${SOLANA_VALIDATOR_RPC_PORT_INTERNAL}"

echo "--- Waiting for Solana validator to be ready ---"
until solana cluster-version --url "$VALIDATOR_URL"; do
  echo "Validator not ready yet, retrying in 2 seconds..."
  sleep 2
done
echo "✅ Solana validator is responsive."

# A small extra delay to ensure the faucet is ready after the RPC server starts.
sleep 2

echo '--- Preparing test environment ---'
# 1. Read the Program ID from the build artifacts.
export PROGRAM_ID=$(cat "$PWD/artifacts/$PROGRAM_IDL_FILENAME" | jq -r .metadata.address)
export W3B2_CONNECTOR__PROGRAM_ID=$PROGRAM_ID
echo "Running tests with PROGRAM_ID=$PROGRAM_ID"

# 2. Patch source files with the correct Program ID, just like in the build script.
echo 'Patching source files for test consistency...'
sed -i -E 's/(declare_id!\s*\(\s*").*("\)\s*;)/\1'"$PROGRAM_ID"'\2/' "$PWD/$PROGRAM_DIR_NAME/src/lib.rs"
sed -i -E 's/('"$PROGRAM_NAME"'\s*=\s*\").*(\")/\1'"$PROGRAM_ID"'\2/' "$PWD/Anchor.toml"

# 3. Create a symlink so that tests can find the compiled .so file.
mkdir -p "$PWD/target/deploy"
ln -sf "$PWD/artifacts/$PROGRAM_SO_FILENAME" "$PWD/target/deploy/$PROGRAM_SO_FILENAME"

echo '--- Running all unit and documentation tests ---'
cargo test --workspace -- --nocapture
echo '--- Running all ignored (integration) tests ---'
cargo test --workspace -- --ignored --nocapture

echo "✅ All tests passed."