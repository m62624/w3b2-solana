#!/bin/bash
set -e

# --- Configuration ---
PROGRAM_IDL_FILENAME="${PROGRAM_IDL_FILENAME:-w3b2_solana_program.idl.json}"
GATEWAY_CONFIG_PATH="${GATEWAY_CONFIG_PATH:-config.docker.toml}"

# The PROGRAM_ID might be empty on the first run, so we read it from the IDL.
if [ -z "$PROGRAM_ID" ]; then
  export PROGRAM_ID=$(cat "$PWD/artifacts/$PROGRAM_IDL_FILENAME" | jq -r .metadata.address)
fi

# Update the connector's PROGRAM_ID in the environment for the gateway process
export W3B2_CONNECTOR__PROGRAM_ID=$PROGRAM_ID
echo "🚀 Starting gateway with PROGRAM_ID=$PROGRAM_ID"

# Execute the gateway binary
"$PWD/target/release/w3b2-solana-gateway" run --config "$PWD/$GATEWAY_CONFIG_PATH"