#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# PostQuantumEVM — End-to-End Demo Script
# ─────────────────────────────────────────────────────────────────────────────
#
# This script demonstrates the full post-quantum blockchain flow:
#   1. Start the PQ node (dev mode)
#   2. Generate ML-DSA-65 keypairs
#   3. Check balances (pre-funded from genesis)
#   4. Send ETH between PQ accounts
#   5. Deploy a smart contract
#   6. Query receipts and chain state
#
# Prerequisites:
#   - pq-reth binary built (cargo build -p pq-reth in pq-reth/)
#   - pq-wallet binary built (cargo build -p pq-wallet-cli in pq-wallet/)
#
# Usage:
#   ./scripts/demo.sh [--skip-build]
#
set -euo pipefail

# ─── Configuration ────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
PQ_RETH_BIN="$ROOT_DIR/pq-reth/target/debug/pq-reth"
PQ_WALLET_BIN="$ROOT_DIR/pq-wallet/target/debug/pq-wallet"
GENESIS="$ROOT_DIR/pq-reth/bin/pq-reth/genesis.json"
DATADIR="/tmp/pqevm-demo/datadir"
KEYSTORE_DIR="/tmp/pqevm-demo/keys"
RPC="http://localhost:8545"
NODE_PID=""

# ─── Colors ───────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info()  { echo -e "${BLUE}[INFO]${NC} $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}   $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
fail()  { echo -e "${RED}[FAIL]${NC} $*"; exit 1; }
header(){ echo -e "\n${GREEN}═══════════════════════════════════════════════════════════${NC}"; echo -e "${GREEN}  $*${NC}"; echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}\n"; }

# ─── Cleanup ──────────────────────────────────────────────────────────────────

cleanup() {
    if [[ -n "$NODE_PID" ]] && kill -0 "$NODE_PID" 2>/dev/null; then
        info "Stopping PQ node (PID $NODE_PID)..."
        kill "$NODE_PID" 2>/dev/null || true
        wait "$NODE_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

# ─── Build (optional) ────────────────────────────────────────────────────────

if [[ "${1:-}" != "--skip-build" ]]; then
    header "Building pq-reth and pq-wallet"
    
    info "Building pq-reth..."
    (cd "$ROOT_DIR/pq-reth" && cargo build -p pq-reth 2>&1 | tail -3)
    ok "pq-reth built"
    
    info "Building pq-wallet..."
    (cd "$ROOT_DIR/pq-wallet" && cargo build -p pq-wallet-cli 2>&1 | tail -3)
    ok "pq-wallet built"
fi

# Verify binaries exist
[[ -x "$PQ_RETH_BIN" ]]  || fail "pq-reth binary not found at $PQ_RETH_BIN"
[[ -x "$PQ_WALLET_BIN" ]] || fail "pq-wallet binary not found at $PQ_WALLET_BIN"

# ─── Step 1: Start PQ Node ───────────────────────────────────────────────────

header "Step 1: Starting PostQuantumEVM Node"

# Clean previous state
rm -rf "$DATADIR" "$KEYSTORE_DIR"
mkdir -p "$DATADIR" "$KEYSTORE_DIR"

info "Starting pq-reth in dev mode (block time: 3s)..."
"$PQ_RETH_BIN" node \
    --dev \
    --dev.block-time 3s \
    --datadir "$DATADIR" \
    --chain "$GENESIS" \
    --http \
    --http.port 8545 \
    --http.api eth,net,web3 \
    --log.stdout.filter error \
    > /tmp/pqevm-demo/node.log 2>&1 &
NODE_PID=$!

info "Waiting for node to start (PID $NODE_PID)..."
for i in $(seq 1 20); do
    if curl -sf "$RPC" -X POST -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' >/dev/null 2>&1; then
        break
    fi
    sleep 1
done

# Verify node is responsive
BLOCK=$(curl -s "$RPC" -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' | \
    python3 -c "import json,sys; print(int(json.load(sys.stdin)['result'], 16))")
ok "Node running — current block: $BLOCK"

CHAIN_ID=$(curl -s "$RPC" -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' | \
    python3 -c "import json,sys; print(int(json.load(sys.stdin)['result'], 16))")
ok "Chain ID: $CHAIN_ID (0x$(printf '%x' $CHAIN_ID))"

# ─── Step 2: Generate ML-DSA-65 Keypairs ─────────────────────────────────────

header "Step 2: Generating ML-DSA-65 Keypairs"

info "Generating sender keypair..."
"$PQ_WALLET_BIN" new --output "$KEYSTORE_DIR/alice.json" --passphrase "alice123"
ALICE=$("$PQ_WALLET_BIN" address --keystore "$KEYSTORE_DIR/alice.json")
ok "Alice: $ALICE"

info "Generating receiver keypair..."
"$PQ_WALLET_BIN" new --output "$KEYSTORE_DIR/bob.json" --passphrase "bob456"
BOB=$("$PQ_WALLET_BIN" address --keystore "$KEYSTORE_DIR/bob.json")
ok "Bob:   $BOB"

# ─── Step 3: Check Pre-funded Balances ───────────────────────────────────────

header "Step 3: Checking Pre-funded Balances"

# Use a pre-funded genesis account as the funding source
FUNDER="0x1aCa481356Ff4F45F84526E4D38e7d63E091E3d0"
FUNDER_BALANCE=$(curl -s "$RPC" -X POST -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBalance\",\"params\":[\"$FUNDER\",\"latest\"],\"id\":1}" | \
    python3 -c "import json,sys; r=json.load(sys.stdin)['result']; print(f'{int(r,16)/1e18:.4f} ETH')")
ok "Funder ($FUNDER): $FUNDER_BALANCE"

info "Note: Alice and Bob are freshly generated — they start with 0 ETH."
info "In production, fund them from genesis or via a faucet."

# ─── Step 4: Send ETH from Pre-funded Account ────────────────────────────────

header "Step 4: Sending PQ Transaction"

# Use the pre-existing funded keystore if available
SENDER_KEYSTORE="/tmp/pq-e2e-test/sender.json"
if [[ -f "$SENDER_KEYSTORE" ]]; then
    info "Using pre-funded sender keystore..."
    SENDER_PASS="test123"
else
    warn "No pre-funded keystore found. Generating new one (won't have balance)."
    SENDER_KEYSTORE="$KEYSTORE_DIR/alice.json"
    SENDER_PASS="alice123"
fi

SENDER_ADDR=$("$PQ_WALLET_BIN" address --keystore "$SENDER_KEYSTORE")
info "Sender: $SENDER_ADDR"
info "Sending 1 ETH to Bob ($BOB)..."

TX_HASH=$("$PQ_WALLET_BIN" send \
    --keystore "$SENDER_KEYSTORE" \
    --passphrase "$SENDER_PASS" \
    --to "$BOB" \
    --value 1000000000000000000 \
    --gas-limit 21000 \
    --rpc "$RPC" 2>&1 | grep "Transaction hash:" | awk '{print $NF}')

if [[ -z "$TX_HASH" ]]; then
    warn "Transaction may have failed (sender might not be funded in genesis)"
    info "Skipping receipt check..."
else
    ok "Transaction sent: $TX_HASH"

    # ─── Step 5: Wait for Receipt ────────────────────────────────────────────
    header "Step 5: Waiting for Transaction Receipt"
    
    info "Polling for receipt..."
    sleep 5
    
    RECEIPT=$(curl -s "$RPC" -X POST -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getTransactionReceipt\",\"params\":[\"$TX_HASH\"],\"id\":1}")
    
    STATUS=$(echo "$RECEIPT" | python3 -c "import json,sys; r=json.load(sys.stdin).get('result'); print(r.get('status','unknown') if r else 'pending')")
    
    if [[ "$STATUS" == "0x1" ]]; then
        ok "Transaction SUCCEEDED!"
        BLOCK_NUM=$(echo "$RECEIPT" | python3 -c "import json,sys; r=json.load(sys.stdin)['result']; print(int(r['blockNumber'],16))")
        GAS_USED=$(echo "$RECEIPT" | python3 -c "import json,sys; r=json.load(sys.stdin)['result']; print(int(r['gasUsed'],16))")
        info "  Block:    $BLOCK_NUM"
        info "  Gas used: $GAS_USED"
    else
        warn "Transaction status: $STATUS"
    fi
    
    # Check Bob's new balance
    BOB_BALANCE=$(curl -s "$RPC" -X POST -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBalance\",\"params\":[\"$BOB\",\"latest\"],\"id\":1}" | \
        python3 -c "import json,sys; r=json.load(sys.stdin)['result']; print(f'{int(r,16)/1e18:.6f} ETH')")
    ok "Bob's balance: $BOB_BALANCE"
fi

# ─── Step 6: Deploy PQHASH Helper Contract ───────────────────────────────────

header "Step 6: Deploying PQHASH Helper Contract"

# The PQHASH helper bytecode (wraps opcode 0x21):
# Runtime: 365f5f37365f215f5260205ff3
# Init:    600d80600e5f395ff3365f5f37365f215f5260205ff3
PQHASH_INIT_CODE="600d80600e5f395ff3365f5f37365f215f5260205ff3"

if [[ -f "$SENDER_KEYSTORE" ]]; then
    info "Deploying PQHASH helper contract..."
    DEPLOY_OUTPUT=$("$PQ_WALLET_BIN" deploy \
        --keystore "$SENDER_KEYSTORE" \
        --passphrase "$SENDER_PASS" \
        --code "$PQHASH_INIT_CODE" \
        --gas-limit 100000 \
        --rpc "$RPC" 2>&1 || true)
    
    DEPLOY_TX=$(echo "$DEPLOY_OUTPUT" | grep "Transaction hash:" | awk '{print $NF}' || true)
    if [[ -n "$DEPLOY_TX" ]]; then
        ok "Deploy tx: $DEPLOY_TX"
        sleep 5
        
        DEPLOY_RECEIPT=$(curl -s "$RPC" -X POST -H "Content-Type: application/json" \
            -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getTransactionReceipt\",\"params\":[\"$DEPLOY_TX\"],\"id\":1}")
        CONTRACT_ADDR=$(echo "$DEPLOY_RECEIPT" | python3 -c "import json,sys; r=json.load(sys.stdin).get('result'); print(r.get('contractAddress','none') if r else 'pending')" 2>/dev/null || echo "pending")
        
        if [[ "$CONTRACT_ADDR" != "none" && "$CONTRACT_ADDR" != "pending" && "$CONTRACT_ADDR" != "null" ]]; then
            ok "PQHASH helper deployed at: $CONTRACT_ADDR"
        else
            info "Contract address: $CONTRACT_ADDR (may still be pending)"
        fi
    else
        warn "Deploy may have failed: $DEPLOY_OUTPUT"
    fi
else
    warn "Skipping deploy (no funded sender)"
fi

# ─── Summary ──────────────────────────────────────────────────────────────────

header "Demo Complete!"

FINAL_BLOCK=$(curl -s "$RPC" -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' | \
    python3 -c "import json,sys; print(int(json.load(sys.stdin)['result'], 16))")

echo ""
echo "  Chain:         PostQuantumEVM (chain_id=$CHAIN_ID)"
echo "  Blocks mined:  $FINAL_BLOCK"
echo "  Signature:     ML-DSA-65 (CRYSTALS-Dilithium)"
echo "  Hash function: SHAKE-256 (addresses, tx hashing)"
echo "  New opcode:    0x21 PQHASH (native SHAKE-256 in EVM)"
echo "  Tx type:       0x50 (PQ transaction envelope)"
echo ""
echo "  Disabled classical precompiles:"
echo "    0x01 ecrecover, 0x06-0x08 BN254, 0x0a KZG, 0x0b-0x13 BLS12-381"
echo ""
echo "  Active PQ precompile:"
echo "    0x0100 ML-DSA-65 verify (50,000 gas)"
echo ""
info "Node log: /tmp/pqevm-demo/node.log"
info "Node still running (PID $NODE_PID). Press Ctrl+C or run 'kill $NODE_PID' to stop."
echo ""

# Keep running until user interrupts
wait "$NODE_PID" 2>/dev/null || true
