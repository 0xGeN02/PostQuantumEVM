#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# PostQuantumEVM — E2E Test Runner
# ─────────────────────────────────────────────────────────────────────────────
#
# Modes:
#   1. Local (default): Starts pq-reth in dev mode, runs E2E validator, cleans up.
#   2. K8s: Assumes a running K8s cluster, applies manifests, runs E2E, optionally cleans.
#   3. External: Runs E2E against an already-running node (no lifecycle management).
#
# Usage:
#   ./e2e/run-e2e.sh                           # Local dev mode
#   ./e2e/run-e2e.sh --mode k8s               # Deploy to K8s and test
#   ./e2e/run-e2e.sh --mode external --rpc http://10.0.0.1:8545
#   ./e2e/run-e2e.sh --readonly                # Skip tx tests (validation only)
#
set -euo pipefail

# ─── Configuration ────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
PQ_RETH_BIN="${PQ_RETH_BIN:-$ROOT_DIR/pq-reth/target/debug/pq-reth}"
PQ_E2E_BIN="${PQ_E2E_BIN:-$ROOT_DIR/pq-wallet/target/debug/pq-e2e}"
GENESIS="$ROOT_DIR/pq-reth/bin/pq-reth/genesis.json"
DATADIR="/tmp/pqevm-e2e/datadir"
# Use the pre-generated E2E keystore (address funded in genesis)
KEYSTORE="$SCRIPT_DIR/fixtures/e2e-keystore.json"
E2E_PASSPHRASE="e2etest"
RPC_URL="http://localhost:8545"
NODE_PID=""
MODE="local"
READONLY=""
EXTRA_ARGS=""
SKIP_BUILD=""

# ─── Parse Arguments ──────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case "$1" in
        --mode)     MODE="$2"; shift 2 ;;
        --rpc)      RPC_URL="$2"; shift 2 ;;
        --readonly) READONLY="--readonly"; shift ;;
        --skip-build) SKIP_BUILD="1"; shift ;;
        --verbose|-v) EXTRA_ARGS="$EXTRA_ARGS --verbose"; shift ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "OPTIONS:"
            echo "  --mode <local|k8s|external>  Deployment mode (default: local)"
            echo "  --rpc <URL>                  RPC endpoint (for external mode)"
            echo "  --readonly                   Skip transaction tests"
            echo "  --skip-build                 Skip building binaries"
            echo "  --verbose, -v                Verbose output"
            echo "  --help, -h                   Show this help"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# ─── Colors ───────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()   { echo -e "${BLUE}[INFO]${NC} $*"; }
ok()     { echo -e "${GREEN}[OK]${NC}   $*"; }
warn()   { echo -e "${YELLOW}[WARN]${NC} $*"; }
fail()   { echo -e "${RED}[FAIL]${NC} $*"; exit 1; }
header() { echo -e "\n${GREEN}═══ $* ═══${NC}\n"; }

# ─── Cleanup ──────────────────────────────────────────────────────────────────

cleanup() {
    if [[ -n "$NODE_PID" ]] && kill -0 "$NODE_PID" 2>/dev/null; then
        info "Stopping PQ node (PID $NODE_PID)..."
        kill "$NODE_PID" 2>/dev/null || true
        wait "$NODE_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

# ─── Build ────────────────────────────────────────────────────────────────────

build_binaries() {
    if [[ -n "$SKIP_BUILD" ]]; then
        info "Skipping build (--skip-build)"
        return
    fi

    header "Building binaries"

    if [[ "$MODE" == "local" ]]; then
        info "Building pq-reth..."
        (cd "$ROOT_DIR/pq-reth" && cargo build -p pq-reth 2>&1 | tail -3)
        ok "pq-reth built"
    fi

    info "Building pq-e2e..."
    (cd "$ROOT_DIR/pq-wallet" && cargo build --bin pq-e2e 2>&1 | tail -3)
    ok "pq-e2e built"

    info "Building pq-wallet..."
    (cd "$ROOT_DIR/pq-wallet" && cargo build --bin pq-wallet 2>&1 | tail -3)
    ok "pq-wallet built"
}

# ─── Local Mode: Start Node ──────────────────────────────────────────────────

start_local_node() {
    header "Starting PostQuantumEVM Node (dev mode)"

    [[ -x "$PQ_RETH_BIN" ]] || fail "pq-reth binary not found at $PQ_RETH_BIN"

    rm -rf "$DATADIR"
    mkdir -p "$DATADIR" "$(dirname $KEYSTORE)"

    info "Starting pq-reth (block time: 3s)..."
    "$PQ_RETH_BIN" node \
        --dev \
        --dev.block-time 3s \
        --datadir "$DATADIR" \
        --chain "$GENESIS" \
        --http \
        --http.port 8545 \
        --http.api eth,net,web3 \
        --log.stdout.filter error \
        > /tmp/pqevm-e2e/node.log 2>&1 &
    NODE_PID=$!

    info "Waiting for node (PID $NODE_PID)..."
    for i in $(seq 1 30); do
        if curl -sf "$RPC_URL" -X POST -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' >/dev/null 2>&1; then
            ok "Node is ready"
            return
        fi
        sleep 1
    done
    fail "Node did not start within 30s (check /tmp/pqevm-e2e/node.log)"
}

# ─── K8s Mode: Deploy to Cluster ─────────────────────────────────────────────

deploy_k8s() {
    header "Deploying to Kubernetes"

    command -v kubectl >/dev/null 2>&1 || fail "kubectl not found"

    info "Applying K8s manifests..."
    kubectl apply -f "$SCRIPT_DIR/k8s/"
    ok "Manifests applied"

    info "Waiting for validators to be ready..."
    kubectl wait --for=condition=ready pod -l app.kubernetes.io/component=validator \
        -n pqevm --timeout=120s
    ok "All validators ready"

    # Get RPC URL
    RPC_URL=$(kubectl get svc pq-rpc -n pqevm -o jsonpath='{.spec.clusterIP}' 2>/dev/null || echo "")
    if [[ -z "$RPC_URL" ]]; then
        # Fallback: port-forward
        info "Starting port-forward to pq-rpc service..."
        kubectl port-forward svc/pq-rpc 8545:8545 -n pqevm &
        sleep 2
        RPC_URL="http://localhost:8545"
    else
        RPC_URL="http://${RPC_URL}:8545"
    fi
    ok "RPC endpoint: $RPC_URL"
}

# ─── Generate Test Keystore ──────────────────────────────────────────────────

generate_keystore() {
    if [[ -n "$READONLY" ]]; then
        return
    fi

    header "Generating Test Keystore"

    PQ_WALLET_BIN="${PQ_WALLET_BIN:-$ROOT_DIR/pq-wallet/target/debug/pq-wallet}"
    [[ -x "$PQ_WALLET_BIN" ]] || fail "pq-wallet binary not found at $PQ_WALLET_BIN"

    if [[ -f "$KEYSTORE" ]]; then
        info "Keystore already exists at $KEYSTORE"
    else
        info "Generating ML-DSA-65 keypair..."
        "$PQ_WALLET_BIN" new --output "$KEYSTORE" --passphrase "e2etest"
        ok "Keystore created: $KEYSTORE"
    fi

    ADDR=$("$PQ_WALLET_BIN" address --keystore "$KEYSTORE")
    info "Test address: $ADDR"
    info "Note: This address needs qETH to run tx tests."
    info "      Use a pre-funded genesis account keystore for full E2E."
}

# ─── Run E2E Validation ──────────────────────────────────────────────────────

run_e2e() {
    header "Running E2E Validation"

    [[ -x "$PQ_E2E_BIN" ]] || fail "pq-e2e binary not found at $PQ_E2E_BIN"

    local cmd="$PQ_E2E_BIN --rpc $RPC_URL"

    if [[ -z "$READONLY" && -f "$KEYSTORE" ]]; then
        cmd="$cmd --keystore $KEYSTORE --passphrase e2etest"
    elif [[ -n "$READONLY" ]]; then
        cmd="$cmd --readonly"
    fi

    cmd="$cmd $EXTRA_ARGS"

    info "Running: $cmd"
    echo ""

    if eval "$cmd"; then
        ok "E2E validation PASSED"
        return 0
    else
        fail "E2E validation FAILED"
    fi
}

# ─── Main ─────────────────────────────────────────────────────────────────────

header "PostQuantumEVM E2E Test — mode: $MODE"

case "$MODE" in
    local)
        build_binaries
        start_local_node
        generate_keystore
        run_e2e
        ;;
    k8s)
        build_binaries
        deploy_k8s
        generate_keystore
        run_e2e
        ;;
    external)
        build_binaries
        generate_keystore
        run_e2e
        ;;
    *)
        fail "Unknown mode: $MODE (expected: local, k8s, external)"
        ;;
esac
