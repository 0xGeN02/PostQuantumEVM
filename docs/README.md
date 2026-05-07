# PostQuantumEVM — Project Documentation

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Components](#components)
- [Cryptographic Primitives](#cryptographic-primitives)
- [Transaction Format](#transaction-format)
- [Gas & Performance](#gas--performance)
- [Deployment](#deployment)
- [Known Limitations](#known-limitations)
- [Considered Alternatives](#considered-alternatives)
- [Documentation Index](#documentation-index)

---

## Overview

**PostQuantumEVM** is a research-grade Ethereum execution client hardened against
quantum adversaries. It replaces every classical elliptic-curve primitive with
NIST-standardized post-quantum algorithms:

| Classical (EVM) | Post-Quantum (PQ-EVM) | Standard |
|---|---|---|
| ECDSA / secp256k1 | ML-DSA-65 (CRYSTALS-Dilithium) | NIST FIPS 204 |
| ECDH / secp256k1 | ML-KEM (CRYSTALS-Kyber) | NIST FIPS 203 |
| Keccak-256 (protocol hash) | SHAKE-256 | NIST FIPS 202 |
| ecrecover precompile | ML-DSA verify precompile (`0x0100`) | — |

The project is built as a **non-invasive extension of [reth](https://github.com/paradigmxyz/reth)**.
All PQ-specific code lives under `pq-reth/crates/pq/` — no upstream reth code is modified.

**Chain ID:** 20561 | **Native Token:** qETH | **Tx Type:** `0x50`

---

## Architecture

```
PostQuantumEVM/
├── pq-reth/                        # Forked reth (git submodule)
│   ├── bin/pq-reth/                # Binary + genesis.json
│   └── crates/pq/
│       ├── reth-pq-primitives      # Core types: PqSignedTransaction, RLP, Compact
│       ├── reth-pq-consensus       # ML-DSA-65 transaction validation
│       ├── reth-pq-precompile      # ML-DSA-65 verify precompile at 0x0100
│       ├── reth-pq-pool            # Mempool with PQ validation + state checks
│       ├── reth-pq-evm             # PqEvmFactory, disabled precompiles, PQHASH
│       ├── reth-pq-node-primitives # PqPrimitives (NodePrimitives impl)
│       ├── reth-pq-node            # PqNode, engine, RPC, payload builder
│       └── reth-pq-poa             # Proof of Authority engine (ML-DSA-65 sealing)
├── ml-lattice-rs/                  # PQ crypto library (git submodule)
│   ├── dilithium/                  # ML-DSA-65 wrapper (FIPS 204)
│   └── kyber/                      # ML-KEM wrapper (FIPS 203)
├── pq-wallet/                      # PQ wallet ecosystem
│   ├── pq-wallet-core/             # Core library (keygen, signer, RPC, tx)
│   ├── pq-wallet-cli/              # CLI binary (pq-wallet)
│   ├── pq-wallet-tui/              # Terminal UI dashboard (pq-tui)
│   ├── pq-chain-seeder/            # Chain seeder for demos (pq-seed)
│   └── pq-e2e-test/                # End-to-end validation (pq-e2e)
├── qiskit-api/                     # Quantum attack simulation (Shor / Grover)
├── contracts/                      # Solidity PQ precompile interfaces (Foundry)
├── benchmarks/                     # Criterion.rs benchmarks (tx encoding)
├── e2e/                            # E2E orchestration + K8s manifests
├── Dockerfile.pq-reth              # Multi-stage Docker build
└── docker-compose.yml              # 3-validator PoA Docker Compose
```

---

## Components

### `ml-lattice-rs` — PQ Crypto Library

Pure-Rust implementation of lattice-based post-quantum algorithms.

| Module | Algorithm | Purpose |
|---|---|---|
| `dilithium` | ML-DSA-65 | Transaction and block signing |
| `kyber` | ML-KEM-768 | Key encapsulation / key exchange |

### `pq-reth` — Ethereum Node

Fork of [reth](https://github.com/paradigmxyz/reth) with PQ extensions.
Consensus is **Proof of Authority (PoA)**: a fixed validator set signs blocks
with ML-DSA-65 in round-robin rotation (5s slots).

### `pq-wallet` — Wallet Ecosystem

| Binary | Purpose |
|--------|---------|
| `pq-wallet` | CLI: keygen, send, sign, balance, call |
| `pq-tui` | 4-tab TUI dashboard with interactive Send/Deploy/Call |
| `pq-seed` | Chain seeder (demo data: transfers + contract) |
| `pq-e2e` | E2E validation (12 scenarios, 6 phases) |

### `qiskit-api` — Quantum Attack Simulation

Python 3.13 / Qiskit service that simulates Shor's algorithm (against ECDSA)
and Grover's algorithm (against hash functions).

### `contracts` — Solidity Interfaces

Foundry-based contracts providing Solidity bindings for the PQ precompiles.

---

## Cryptographic Primitives

### Key and Signature Sizes

| | Classical Ethereum | PostQuantumEVM | Ratio |
|---|---|---|---|
| Private key | 32 B | 4 032 B | 126x |
| Public key | 64 B | 1 952 B | 30x |
| Signature | 65 B | 3 309 B | 51x |
| Address | 20 B | 20 B | **1x** |

Address derivation: `shake256(pk_bytes, 32)[12..]`

### Verification Performance

| Operation | Classical | PQ | Ratio |
|---|---|---|---|
| Verification | 49.2 µs | 42.0 µs | **PQ is 14% faster** |
| Signing | 41.4 µs | 342.8 µs | 8.3x slower |
| Key generation | 35.1 µs | 208.5 µs | 5.9x slower |

ML-DSA-65 verification is faster than ecrecover due to CPU vectorization (AVX2/NEON).
Signing is client-side only and not on the consensus-critical path.

---

## Transaction Format

PQ transactions use EIP-2718 type **`0x50`** (avoids collision with EIP-7702).

```
0x50 || RLP([chain_id, nonce, gas_price, gas_limit, to, value, input,
             signature (3309 B), public_key (1952 B)])
```

| Field | Classical tx | PQ tx |
|---|---|---|
| Signature | 65 B | 3 309 B |
| Public key | 0 B (recovered) | 1 952 B |
| Total size | ~110 B | ~5 314 B |

---

## Gas & Performance

Full analysis in [GAS_COST_ANALYSIS.md](GAS_COST_ANALYSIS.md).

| Parameter | Classical | PQ | Note |
|---|---|---|---|
| Simple transfer gas | ~21 976 | ~42 892 | +95% |
| Precompile verify gas | 3 000 | 3 450 | +15% |
| Max transfers/block | ~1 638 | ~839 | 51% capacity |
| Block bandwidth | ~100 KB | ~4.5 MB | 45x |

---

## Deployment

### Development (single node)

```bash
cd pq-reth
cargo run -p pq-reth --bin pq-reth -- node \
  --chain bin/pq-reth/genesis.json --dev --dev.block-time 5s \
  --http --http.api eth,net,web3
```

### Docker Compose (3-validator PoA)

```bash
./scripts/generate-validator-keys.sh
docker compose build
docker compose up -d
```

### Kubernetes

```bash
kubectl apply -f e2e/k8s/
```

See [NODE.md](NODE.md) for full deployment instructions.

---

## Known Limitations

### 1. Signature Size Bottleneck

The ~48x increase in transaction size (5.3 KB vs ~110 B) is the primary constraint:

- **Block throughput:** ~50% fewer transactions per block
- **P2P bandwidth:** Blocks are ~45x larger
- **Mempool memory:** Each pending tx consumes ~50x more RAM
- **Storage growth:** ~130 GB/day at 1 block/3s vs ~2.9 GB classically

### 2. No Signature Aggregation

BLS12-381 aggregation (used in Ethereum PoS) has no ML-DSA equivalent.

### 3. Non-Recoverable Signatures

The 1952-byte public key must be embedded in every transaction.

### 4. No Backward Compatibility

Classic ECDSA transactions are not supported (intentional — no classical attack surface).

---

## Considered Alternatives

| Approach | Why Not Adopted |
|----------|----------------|
| zkSNARKs (Groth16/PLONK) | Rely on elliptic curves → quantum-vulnerable |
| ZK Rollup over PQ L1 | Proof system uses BN254 → not quantum-safe |
| STARKs | Proof sizes large, recursive STARKs not production-ready |
| Optimistic Rollup | 7-day withdrawal delay, separate infrastructure |
| FALCON aggregation | No standardized scheme, timing side channels |

---

## Documentation Index

| Document | Description |
|---|---|
| [CONSENSUS.md](CONSENSUS.md) | PoA consensus mechanism design |
| [GAS_COST_ANALYSIS.md](GAS_COST_ANALYSIS.md) | Benchmarks, gas pricing, throughput |
| [NODE.md](NODE.md) | Node architecture, Docker/K8s deployment |
| [WALLET.md](WALLET.md) | Wallet, TUI, keystore, chain seeder |
| [e2e/k8s/README.md](../e2e/k8s/README.md) | Kubernetes deployment manifests |
