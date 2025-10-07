#!/bin/bash
set -e

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
echo "  - Updating declare_id! in $LIB_RS_PATH"
echo "    -> Before patch:"
grep --color=never 'declare_id!' "$LIB_RS_PATH" || echo "    -> 'declare_id!' not found."
python3 -c "
import re, sys, os
path = '$LIB_RS_PATH'
content = open(path).read()
program_id = os.environ.get('PROGRAM_ID')
# Match the whole macro call including the semicolon to ensure valid syntax
new_content = re.sub(r'declare_id!\s*\(\s*".*?"\s*\);', f'declare_id!(\"{program_id}\");', content, count=1)
open(path, 'w').write(new_content)
"
echo "    -> After patch:"
grep --color=never 'declare_id!' "$LIB_RS_PATH" || echo "    -> 'declare_id!' not found."

# Patch Anchor.toml
echo "  - Updating program ID in $ANCHOR_TOML_PATH"
echo "    -> Before patch:"
cat "$ANCHOR_TOML_PATH"
python3 -c "
import toml, os
path = '$ANCHOR_TOML_PATH'
data = toml.load(path)
program_id = os.environ.get('PROGRAM_ID')
updated = False

if 'programs' in data:
    for cluster, programs_in_cluster in data['programs'].items():
        if 'w3b2_solana_program' in programs_in_cluster:
            print(f'    -> Found and updating in [programs.{cluster}]')
            data['programs'][cluster]['w3b2_solana_program'] = program_id
            updated = True
            break

if not updated:
    print('    -> Key not found, adding to [programs.localnet]')
    data.setdefault('programs', {}).setdefault('localnet', {})
    data['programs']['localnet']['w3b2_solana_program'] = program_id

toml.dump(data, open(path, 'w'))
"
echo "    -> After patch:"
cat "$ANCHOR_TOML_PATH"
echo "✅ Source files patched."


# 5. Build the program
echo "🚀 Building Anchor program ($PROGRAM_DIR)..."
# We build from within the program directory as is standard for Anchor.
# This generates the .so binary and the IDL.
(cd "$PROGRAM_DIR" && anchor build)
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