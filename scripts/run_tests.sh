#!/bin/bash
set -e

# --- Configuration ---
PROGRAM_NAME="${PROGRAM_NAME:-w3b2_solana_program}"
PROGRAM_DIR_NAME="${PROGRAM_DIR_NAME:-w3b2-solana-program}"
PROGRAM_IDL_FILENAME="${PROGRAM_IDL_FILENAME:-w3b2_solana_program.idl.json}"
PROGRAM_SO_FILENAME="${PROGRAM_SO_FILENAME:-w3b2_solana_program.so}"

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