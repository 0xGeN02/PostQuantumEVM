#!/usr/bin/env bash
# generate-validator-keys.sh — Generate ML-DSA-65 validator keys for PoA
#
# Requires: pq-wallet binary (from pq-wallet/ directory)
#
# Generates 3 validator keypairs and outputs a poa-config.json for each node.
# Usage: ./scripts/generate-validator-keys.sh [num_validators]

set -euo pipefail

NUM_VALIDATORS="${1:-3}"
SLOT_TIME_MS="${SLOT_TIME_MS:-5000}"
OUTPUT_DIR="scripts/validator-keys"

echo "=== PostQuantumEVM PoA Key Generation ==="
echo "Generating ${NUM_VALIDATORS} ML-DSA-65 validator keys..."
echo ""

mkdir -p "$OUTPUT_DIR"

# Arrays to hold generated data
declare -a ADDRESSES
declare -a PUBLIC_KEYS

for i in $(seq 1 "$NUM_VALIDATORS"); do
    echo "--- Validator $i ---"
    
    # Generate a keypair using pq-wallet
    # Output: JSON with { signing_key, verifying_key, address }
    KEYFILE="${OUTPUT_DIR}/validator-${i}.json"
    
    if command -v pq-wallet &>/dev/null; then
        pq-wallet keygen --output "$KEYFILE"
        ADDRESS=$(jq -r '.address' "$KEYFILE")
        PUBKEY=$(jq -r '.verifying_key' "$KEYFILE")
    else
        # Fallback: generate placeholder keys (replace with real generation)
        echo "WARNING: pq-wallet not found. Using placeholder keys."
        echo "Build pq-wallet first: cargo build -p pq-wallet --release"
        ADDRESS="$(printf '%040x' $i)"
        PUBKEY="<generate-with-pq-wallet>"
    fi
    
    ADDRESSES+=("$ADDRESS")
    PUBLIC_KEYS+=("$PUBKEY")
    echo "  Address: 0x${ADDRESS}"
    echo "  Key file: ${KEYFILE}"
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
echo "To start PoA nodes:"
echo "  PQ_POA_CONFIG=${OUTPUT_DIR}/poa-config-1.json pq-reth node --dev --http"
