# PostQuantumEVM Node Documentation

## Overview

PostQuantumEVM is a post-quantum resistant Ethereum execution client built as a non-invasive extension of [reth](https://github.com/paradigmxyz/reth). It replaces ECDSA/secp256k1 entirely with **ML-DSA-65 (CRYSTALS-Dilithium)** for transaction signing and verification, uses **SHAKE-256** for all protocol-level hashing, and disables all classical elliptic curve precompiles vulnerable to quantum attacks.

**Chain ID:** 20561 | **Native Token:** qETH | **Tx Type:** `0x50` | **Gas Limit:** 30M

---

## Architecture

```
pq-reth/
├── bin/pq-reth/
│   ├── main.rs             # Entry point (PqNode::launch)
│   └── genesis.json        # Chain spec (chain_id 20561, pre-funded accounts)
└── crates/pq/
    ├── reth-pq-primitives  # PqSignedTransaction, PqSigner, RLP codec, Compact codec
    ├── reth-pq-consensus   # ML-DSA-65 transaction validation
    ├── reth-pq-precompile  # ML-DSA-65 verify precompile at 0x0100
    ├── reth-pq-evm         # PqEvmFactory, PQHASH opcode, disabled precompiles
    ├── reth-pq-pool        # PqPoolValidator (sig verify + state checks)
    ├── reth-pq-node-primitives # PqPrimitives (NodePrimitives impl)
    ├── reth-pq-node        # PqNode, engine, RPC, payload builder
    └── reth-pq-poa         # PoA engine (ML-DSA-65 block sealing)
```

### Integration Strategy

All PQ code lives inside the reth workspace as new crates under `crates/pq/`. No upstream reth code is modified. This enables clean rebasing against upstream reth releases.

---

## Running the Node

### Single Node (Dev Mode)

```bash
cd pq-reth
cargo run -p pq-reth --bin pq-reth -- node \
  --chain bin/pq-reth/genesis.json \
  --dev \
  --dev.block-time 5s \
  --http \
  --http.addr 0.0.0.0 \
  --http.port 8545 \
  --http.api eth,net,web3,admin
```

**Data directory:** `~/.local/share/reth/20561/`

> To reset state (e.g., after genesis changes): `rm -rf ~/.local/share/reth/20561`

### Docker (Single Node)

```bash
docker build -f Dockerfile.pq-reth -t pqevm-node:latest .
docker run -p 8545:8545 pqevm-node:latest
```

### Docker Compose (3-Validator PoA)

```bash
# 1. Generate validator keys
./scripts/generate-validator-keys.sh

# 2. Build and launch
docker compose build
docker compose up -d

# 3. Check status
docker compose logs -f pq-validator-1

# RPC endpoints:
#   Validator 1: http://localhost:8545
#   Validator 2: http://localhost:8546
#   Validator 3: http://localhost:8547
```

### Kubernetes (Production-like)

```bash
# Prerequisites: kubectl configured, Docker image pushed to registry

# 1. Apply manifests
kubectl apply -f e2e/k8s/00-namespace.yaml
kubectl apply -f e2e/k8s/01-genesis-configmap.yaml
kubectl apply -f e2e/k8s/02-poa-configs.yaml
kubectl apply -f e2e/k8s/03-services.yaml
kubectl apply -f e2e/k8s/04-validators-statefulset.yaml

# 2. Check pods
kubectl get pods -n pqevm

# 3. Access RPC (LoadBalancer)
kubectl get svc pq-rpc -n pqevm
```

See [e2e/k8s/README.md](../e2e/k8s/README.md) for full details.

---

## Transaction Format

### Type Envelope

PQ transactions use EIP-2718 type **`0x50`** (`'P'` for Post-Quantum). This avoids
collision with EIP-7702 (type 4) and maps to `TransactionType::Custom` in revm.

### Wire Format

```
0x50 || RLP([chain_id, nonce, gas_price, gas_limit, to, value, input, signature, public_key])
```

### Unsigned Transaction Fields

| Field | Type | Description |
|-------|------|-------------|
| `chain_id` | `u64` | Chain identifier (20561) |
| `nonce` | `u64` | Sender's transaction count |
| `gas_price` | `u128` | Gas price in wei |
| `gas_limit` | `u64` | Maximum gas for execution |
| `to` | `Option<Address>` | Recipient (None = contract creation) |
| `value` | `u128` | Value in wei |
| `input` | `Bytes` | Calldata / contract init code |

### Signing Hash

All protocol-level hashing uses SHAKE-256 (XOF):

```
signing_hash = shake256(
    0x50                       ||    // 1 byte (tx type)
    chain_id                   ||    // 8 bytes, big-endian
    nonce                      ||    // 8 bytes, big-endian
    gas_price                  ||    // 16 bytes, big-endian
    gas_limit                  ||    // 8 bytes, big-endian
    to_flag                    ||    // 1 byte (0x01 present, 0x00 absent)
    [to_address]               ||    // 20 bytes (only if to_flag = 0x01)
    value                      ||    // 16 bytes, big-endian
    input                            // variable length
, 32)  → 32 bytes output
```

### Transaction Hash

```
tx_hash = shake256(0x50 || signing_hash || signature_bytes || public_key_bytes, 32)
```

### Sender Recovery

ML-DSA signatures are **not recoverable** — the public key is embedded in the transaction.
The sender address is derived as:

```
sender = shake256(public_key_bytes, 32)[12..]
```

---

## Consensus

### Proof of Authority (PoA)

PostQuantumEVM uses a **round-robin PoA** mechanism with ML-DSA-65 block sealing:

- Fixed validator set (configured in `poa-config.json`)
- Round-robin block production based on `block_number % num_validators`
- Each block includes an ML-DSA-65 signature (seal) over the block hash
- Slot time: 5 seconds (configurable)

#### PoA Configuration

```json
{
  "slot_time_ms": 5000,
  "local_address": "0x<validator-address>",
  "validators": [
    {
      "address": "0x<address-1>",
      "public_key": "0x<ml-dsa-65-public-key-hex>"
    },
    {
      "address": "0x<address-2>",
      "public_key": "0x<ml-dsa-65-public-key-hex>"
    },
    {
      "address": "0x<address-3>",
      "public_key": "0x<ml-dsa-65-public-key-hex>"
    }
  ]
}
```

Set via environment variable: `PQ_POA_CONFIG=/path/to/poa-config.json`

### Fee Model

Standard EIP-1559:
- Base fee: burned
- Priority fee: paid to the block-producing validator
- No block reward
- Gas limit: follows parent block (1/1024 rule)

---

## EVM Changes

### New Opcode

| Opcode | Name | Stack | Description | Gas |
|--------|------|-------|-------------|-----|
| `0x21` | **PQHASH** | `(offset, length) → hash` | SHAKE-256 over memory region → 32 bytes | 30 + 6/word |

### Disabled Precompiles (14)

All classical elliptic curve precompiles are disabled:

| Address | Name | Reason |
|---------|------|--------|
| `0x01` | ecrecover | ECDSA — broken by Shor's |
| `0x06`–`0x08` | BN254 (ecAdd/Mul/Pairing) | Classical curve |
| `0x0a` | point_evaluation (KZG) | BLS12-381 DLP |
| `0x0b`–`0x13` | BLS12-381 (all 9) | Classical curve |

### Kept Precompiles (quantum-safe)

| Address | Name | Justification |
|---------|------|---------------|
| `0x02` | SHA-256 | Hash — Grover reduces to 128-bit (sufficient) |
| `0x03` | RIPEMD-160 | Hash function |
| `0x04` | Identity | Data copy — no crypto |
| `0x05` | ModExp | Pure arithmetic |
| `0x09` | Blake2f | Hash compression |

### New Precompiles

| Address | Name | Input | Output | Gas |
|---------|------|-------|--------|-----|
| `0x0100` | **pq_verify** | `msg_hash(32) \|\| sig(3309) \|\| pk(1952)` | `0x01` or `0x00` | 50,000 |
| `0x0101` | **pq_batch_verify** | `N(4) \|\| [hash+sig+pk] × N` | `0x01` or `0x00` | ~40k × N × 0.7 |
| `0x0102` | **pq_decapsulate** | `ct(1088) \|\| dk(2400)` | `shared_secret(32)` | TBD |

---

## Node Components

| Component | Purpose |
|-----------|---------|
| `PqNode` | Top-level node type (NodeTypes + Node trait) |
| `PqEvmConfig` | EVM with PQ precompiles + PQHASH opcode |
| `PqEvmFactory` | Creates EVM instances with custom precompile set |
| `PqPoolValidator` | Mempool validator: ML-DSA-65 sig check + nonce/balance |
| `PqPoaConsensus` | Block seal verification (ML-DSA-65) |
| `PoaMiningStream` | Round-robin block production + sealing |
| `PqPayloadBuilder` | Pulls PQ txs from pool, executes, builds blocks |
| `PqRpcTxConverter` | JSON-RPC transaction serialization |
| `PqReceiptBuilder` | Receipt construction for PQ tx type |

---

## Network Configuration

### Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 30303 | TCP+UDP | P2P (devp2p RLPx + discovery) |
| 8545 | TCP | HTTP JSON-RPC |
| 8546 | TCP | WebSocket JSON-RPC |
| 9001 | TCP | Metrics (Prometheus) |

### Genesis

The genesis file (`bin/pq-reth/genesis.json`) configures:
- Chain ID: 20561
- Gas limit: 30,000,000 (30M)
- Pre-funded accounts: 11 addresses with 10,000 qETH each
- All hardforks enabled through Prague

---

## Solidity Integration

### Calling the ML-DSA Precompile

```solidity
library PQVerify {
    address constant PQ_VERIFY = address(0x0100);

    function verify(
        bytes32 msgHash,
        bytes calldata signature,   // 3309 bytes
        bytes calldata publicKey    // 1952 bytes
    ) internal view returns (bool) {
        bytes memory input = abi.encodePacked(msgHash, signature, publicKey);
        (bool ok, bytes memory result) = PQ_VERIFY.staticcall(input);
        return ok && result.length == 1 && result[0] == 0x01;
    }
}
```

### Breaking Changes for Solidity Developers

| Pattern | Status | Alternative |
|---------|--------|-------------|
| `ecrecover(hash, v, r, s)` | Reverts | Use `PQVerify.verify()` |
| OpenZeppelin `ECDSA.recover()` | Broken | Use PQ signature checker |
| ERC-2612 `permit()` | Broken | Implement PQ-based permit |
| EIP-712 typed data signing | Broken | Use ML-DSA over typed hash |
| BLS signature aggregation | Broken | Not available |
| ZK proof verification (Groth16) | Broken | Requires PQ-friendly ZK |
