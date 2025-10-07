#!/bin/bash
set -e

# --- Pre-flight Check for Docker Permissions ---
# Ensure the artifacts directory is owned by the current user.
# This is necessary because Docker might create the bind-mounted directory as root.
mkdir -p artifacts
sudo chown -R "$(id -u):$(id -g)" artifacts

# --- Configuration ---
DEFAULT_KEYPAIR_PATH="/keys/my_program-keypair.json"
PROGRAM_KEYPAIR_PATH="${PROGRAM_KEYPAIR_PATH:-$DEFAULT_KEYPAIR_PATH}"
PROGRAM_DIR="w3b2-solana-program"
LIB_RS_PATH="$PROGRAM_DIR/src/lib.rs"
ANCHOR_TOML_PATH="Anchor.toml"
IDL_PATH_TEMPLATE="target/idl/w3b2_solana_program.json"
DEPLOY_SO_PATH="target/deploy/w3b2_solana_program.so"

# --- Helper Functions ---
print_help() {
    echo "Usage: $0 [MODE]"
    echo "Modes:"
    echo "  --build-only    Build the Anchor program and update IDL."
    echo "  --deploy        Build (if needed) and deploy the program to a validator."
    echo "  --help          Show this help message."
}

# --- Main Logic ---

# 1. Parse command-line arguments
MODE="--build-only" # Default mode
if [ "$1" ]; then
    MODE=$1
fi

if [[ "$MODE" == "--help" ]]; then
    print_help
    exit 0
fi

# 2. Ensure keypair exists
if [ ! -f "$PROGRAM_KEYPAIR_PATH" ]; then
    echo "Program keypair not found at $PROGRAM_KEYPAIR_PATH. Creating a new one..."
    mkdir -p "$(dirname "$PROGRAM_KEYPAIR_PATH")"
    solana-keygen new --no-passphrase -o "$PROGRAM_KEYPAIR_PATH"
    echo "New keypair created."
fi

# 3. Get Program ID
export PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_KEYPAIR_PATH")
echo "🔑 Program ID: $PROGRAM_ID"

# 4. Patch source files before build
echo "🔄 Patching source files with Program ID..."

# Patch lib.rs
echo "  - Patching declare_id! in $LIB_RS_PATH"
TEMP_LIB_RS=$(mktemp)
cp "$LIB_RS_PATH" "$TEMP_LIB_RS"

python3 -c "
import re, sys, os
path = '$LIB_RS_PATH'
content = open(path).read()
program_id = os.environ.get('PROGRAM_ID')
new_content = re.sub(r'declare_id!\s*\(\s*".*?"\s*\);', f'declare_id!(\"{program_id}\");', content, count=1)
open(path, 'w').write(new_content)
"
echo "    -> Patch diff:"
diff --color=always -u "$TEMP_LIB_RS" "$LIB_RS_PATH" || true
rm "$TEMP_LIB_RS"

# Patch Anchor.toml
echo "  - Patching program ID in $ANCHOR_TOML_PATH"
# Create a temporary copy to generate a diff against
TEMP_ANCHOR_TOML=$(mktemp)
cp "$ANCHOR_TOML_PATH" "$TEMP_ANCHOR_TOML"

# Use sed for an in-place replacement to preserve formatting and comments.
# This command finds the line with `w3b2_solana_program = "..."` and replaces the quoted value.
sed -i -E "s/(w3b2_solana_program\s*=\s*\").*(\")/\1$PROGRAM_ID\2/" "$ANCHOR_TOML_PATH"

# Show the diff, using --color=always to force color output.
# The `|| true` prevents the script from exiting if diff finds changes (non-zero exit code).
echo "    -> Patch diff:"
diff --color=always -u "$TEMP_ANCHOR_TOML" "$ANCHOR_TOML_PATH" || true

# Clean up the temporary file
rm "$TEMP_ANCHOR_TOML"
echo "✅ Source files patched."


# 5. Build the program
echo "🚀 Building Anchor workspace..."
# Since this is a workspace, running `anchor build` from the root
# will build all member programs and place artifacts in the root `target/` directory.
anchor build
echo "✅ Anchor program build successful."

# 6. Update IDL with the correct address and move artifacts
ARTIFACTS_DIR="artifacts"
mkdir -p "$ARTIFACTS_DIR"

echo "🔄 Finalizing artifacts..."
if [ -f "$IDL_PATH_TEMPLATE" ]; then
    # Update IDL metadata with the correct address
    jq ".metadata.address = env.PROGRAM_ID" "$IDL_PATH_TEMPLATE" > /tmp/idl.json

    # Move updated IDL to artifacts
    mv /tmp/idl.json "$ARTIFACTS_DIR/w3b2_solana_program.json"
    echo "✅ IDL moved to $ARTIFACTS_DIR/"
else
    echo "⚠️ Warning: IDL file not found at $IDL_PATH_TEMPLATE. Skipping."
fi

if [ -f "$DEPLOY_SO_PATH" ]; then
    # Move program binary to artifacts
    cp "$DEPLOY_SO_PATH" "$ARTIFACTS_DIR/"
    echo "✅ Program binary moved to $ARTIFACTS_DIR/"
else
    echo "⚠️ Warning: Program binary not found at $DEPLOY_SO_PATH. Skipping."
fi


echo "✅ Build complete"
echo "PROGRAM_ID: $PROGRAM_ID"


# 7. Deploy if requested
if [[ "$MODE" == "--deploy" ]]; then
    SOLANA_URL=${SOLANA_RPC_URL:-http://localhost:8899}

    echo "⏳ Waiting for validator at $SOLANA_URL..."
    for i in {1..15}; do
      if solana --url "$SOLANA_URL" ping --no-address-labels > /dev/null 2>&1; then
        echo "✅ Validator is ready."
        break
      fi
      echo "  ...attempt $i, still waiting."
      sleep 1
      if [ $i -eq 15 ]; then
        echo "❌ Validator not available after 15 seconds. Exiting."
        exit 1
      fi
    done

    echo "🚀 Deploying program..."
    solana program deploy "$DEPLOY_SO_PATH" \
        --program-id "$PROGRAM_KEYPAIR_PATH" \
        --url "$SOLANA_URL"

    echo "✅ Program deployed successfully"
    echo "PROGRAM_ID: $PROGRAM_ID"
fi

exit 0