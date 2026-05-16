#!/usr/bin/env bash
# generate-validator-keys.sh — Generate ML-DSA-65 validator keys for PoA
#
# Requires: pq-wallet binary (from pq-wallet/ directory)
#
# Generates N validator keypairs and outputs:
#   - One keystore per validator (encrypted)
#   - One poa-config-N.json per node (for PQ_POA_CONFIG)
#   - One seed-N.hex per node (for PQ_VALIDATOR_SK)
#
# Usage: ./scripts/generate-validator-keys.sh [num_validators]

set -euo pipefail

NUM_VALIDATORS="${1:-3}"
SLOT_TIME_MS="${SLOT_TIME_MS:-5000}"
PASSPHRASE="${PASSPHRASE:-validator}"
OUTPUT_DIR="scripts/validator-keys"

echo "=== PostQuantumEVM PoA Key Generation ==="
echo "Generating ${NUM_VALIDATORS} ML-DSA-65 validator keys..."
echo "Passphrase: ${PASSPHRASE}"
echo ""

# Locate pq-wallet binary
if command -v pq-wallet &>/dev/null; then
    WALLET_BIN="pq-wallet"
elif [ -f "target/release/pq-wallet" ]; then
    WALLET_BIN="./target/release/pq-wallet"
elif [ -f "pq-wallet/target/release/pq-wallet" ]; then
    WALLET_BIN="./pq-wallet/target/release/pq-wallet"
elif [ -f "pq-wallet/target/debug/pq-wallet" ]; then
    WALLET_BIN="./pq-wallet/target/debug/pq-wallet"
else
    echo "ERROR: pq-wallet binary not found."
    echo "Build it first:"
    echo "  cd pq-wallet && cargo build --release -p pq-wallet-cli"
    exit 1
fi

echo "Using wallet: ${WALLET_BIN}"
echo ""

mkdir -p "$OUTPUT_DIR"

# Arrays to hold generated data
declare -a ADDRESSES
declare -a PUBLIC_KEYS
declare -a SEEDS

for i in $(seq 1 "$NUM_VALIDATORS"); do
    echo "--- Validator $i ---"

    KEYFILE="${OUTPUT_DIR}/validator-${i}.json"
    SEEDFILE="${OUTPUT_DIR}/seed-${i}.hex"

    # Generate keypair
    $WALLET_BIN new --output "$KEYFILE" --passphrase "$PASSPHRASE"

    # Extract address and public key (no decryption needed)
    ADDRESS=$(jq -r '.address' "$KEYFILE" | sed 's/^0x//')
    PUBKEY=$(jq -r '.public_key' "$KEYFILE")

    # Export seed for PQ_VALIDATOR_SK
    SEED=$($WALLET_BIN export-seed --keystore "$KEYFILE" --passphrase "$PASSPHRASE")
    echo "$SEED" > "$SEEDFILE"
    chmod 600 "$SEEDFILE"

    ADDRESSES+=("$ADDRESS")
    PUBLIC_KEYS+=("$PUBKEY")
    SEEDS+=("$SEED")

    echo "  Address:  0x${ADDRESS}"
    echo "  Key file: ${KEYFILE}"
    echo "  Seed:     ${SEEDFILE}"
    echo ""
done

# Generate poa-config.json for each validator
echo "=== Generating PoA configs ==="
for i in $(seq 1 "$NUM_VALIDATORS"); do
    idx=$((i - 1))
    CONFIG_FILE="${OUTPUT_DIR}/poa-config-${i}.json"

    # Build validators JSON array
    VALIDATORS="["
    for j in $(seq 0 $((NUM_VALIDATORS - 1))); do
        if [ "$j" -gt 0 ]; then VALIDATORS+=","; fi
        VALIDATORS+=$(cat <<EOF

    {
      "address": "0x${ADDRESSES[$j]}",
      "public_key": "0x${PUBLIC_KEYS[$j]}"
    }
EOF
)
    done
    VALIDATORS+=$'\n  ]'

    cat > "$CONFIG_FILE" <<EOF
{
  "slot_time_ms": ${SLOT_TIME_MS},
  "local_address": "0x${ADDRESSES[$idx]}",
  "validators": ${VALIDATORS}
}
EOF

    echo "  Node $i config: ${CONFIG_FILE}"
done

echo ""
echo "=== Done ==="
echo ""
echo "To start a single PoA node (validator 1):"
echo "  PQ_POA_CONFIG=${OUTPUT_DIR}/poa-config-1.json \\"
echo "  PQ_VALIDATOR_SK=\$(cat ${OUTPUT_DIR}/seed-1.hex) \\"
echo "  pq-reth node --dev --dev.block-time 2s --http --http.addr 0.0.0.0 \\"
echo "    --chain bin/pq-reth/genesis.json"
