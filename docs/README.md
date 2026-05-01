# PostQuantumEVM — Project Documentation

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Components](#components)
- [Cryptographic Primitives](#cryptographic-primitives)
- [Transaction Format](#transaction-format)
- [Gas & Performance](#gas--performance)
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
| Keccak-256 | SHAKE-256 | NIST FIPS 202 |
| ecrecover precompile | ML-DSA verify precompile (`0x0100`) | — |

The project is built as a **non-invasive extension of [reth](https://github.com/paradigmxyz/reth)**.
All PQ-specific code lives under `pq-reth/crates/pq/` — no upstream reth code is modified,
which allows clean rebasing against future reth releases.

---

## Architecture

```
PostQuantumEVM/
├── pq-reth/                        # Forked reth (git submodule)
│   └── crates/pq/
│       ├── reth-pq-primitives      # Core types: PqSignedTransaction, PqSigner, RLP, Compact
│       ├── reth-pq-consensus       # ML-DSA-65 transaction validation
│       ├── reth-pq-precompile      # ML-DSA-65 verify precompile at 0x0100
│       ├── reth-pq-pool            # Mempool with PQ transaction validation
│       ├── reth-pq-evm             # PqEvmFactory, disabled classical precompiles
│       ├── reth-pq-node-primitives # PqPrimitives (NodePrimitives impl)
│       ├── reth-pq-node            # PqNode, engine validator, RPC, payload builder
│       └── reth-pq-poa             # Proof of Authority engine (ML-DSA-65 block sealing)
├── ml-lattice-rs/                  # PQ crypto library
│   ├── dilithium/                  # ML-DSA-65 wrapper (FIPS 204)
│   └── kyber/                      # ML-KEM wrapper (FIPS 203)
├── pq-wallet/                      # PQ wallet (CLI + core library)
├── qiskit-api/                     # Quantum attack simulation (Shor / Grover via Qiskit)
├── contracts/                      # Solidity PQ precompile interfaces and tests
└── benchmarks/                     # Criterion.rs benchmarks (crypto ops, tx encoding)
```

---

## Components

### `ml-lattice-rs` — PQ Crypto Library

Pure-Rust implementation of lattice-based post-quantum algorithms.
Wraps [`ml-dsa`](https://crates.io/crates/ml-dsa) and [`ml-kem`](https://crates.io/crates/ml-kem).

| Module | Algorithm | Purpose |
|---|---|---|
| `dilithium` | ML-DSA-65 | Transaction and block signing |
| `kyber` | ML-KEM-768 | Key encapsulation / key exchange |

### `pq-reth` — Ethereum Node

Fork of [reth](https://github.com/paradigmxyz/reth) with PQ extensions.
Consensus is **Proof of Authority (PoA)**: a fixed validator set signs blocks
with ML-DSA-65 in round-robin rotation. The Engine API is retained for tooling
compatibility.

### `pq-wallet` — Key Management

CLI wallet and library crate for generating ML-DSA-65 keypairs, deriving
addresses, and constructing signed PQ transactions.

### `qiskit-api` — Quantum Attack Simulation

Python 3.13 / Qiskit service that simulates Shor's algorithm (against ECDSA)
and Grover's algorithm (against hash functions), used to empirically demonstrate
why classical primitives are insufficient.

### `contracts` — Solidity Interfaces

Foundry-based contracts providing Solidity bindings for the PQ precompiles
(`PQVerify.sol`, `PQHash.sol`, `PQMultiSig.sol`, `PQAccessControl.sol`) and
Forge gas benchmarks.

---

## Cryptographic Primitives

### Key and Signature Sizes

| | Classical Ethereum | PostQuantumEVM | Ratio |
|---|---|---|---|
| Private key | 32 B | 4 032 B | 126x |
| Public key | 64 B | 1 952 B | 30x |
| Signature | 65 B | 3 309 B | 51x |
| Address | 20 B | 20 B | **1x** |

The 20-byte address format is preserved. Derivation changes from
`keccak256(pk)[12..]` to `shake256(pk)[12..]` to maintain cryptographic
consistency with ML-DSA's internal use of SHAKE-256.

### Verification Performance

Counter-intuitively, **ML-DSA-65 verification is faster than `ecrecover`**:

| Operation | Classical | PQ | Ratio |
|---|---|---|---|
| Verification | 49.2 µs | 42.0 µs | **PQ is 14% faster** |
| Signing | 41.4 µs | 342.8 µs | 8.3x slower |
| Key generation | 35.1 µs | 208.5 µs | 5.9x slower |

ML-DSA-65 verification benefits from modern CPU vectorization (AVX2/NEON).
Signing and key generation are only performed client-side and are not on the
consensus-critical path.

---

## Transaction Format

PQ transactions use EIP-2718 type **`0x50`** (avoids collision with EIP-7702).

```
0x50 || RLP([nonce, to, value, gas_limit, gas_price, input, chain_id,
             signature (3309 B), public_key (1952 B)])
```

Because ML-DSA-65 signatures are not recoverable (unlike ECDSA), the public key
is embedded in every transaction. The sender address is derived on-chain from the
embedded public key.

| Field | Classical tx | PQ tx |
|---|---|---|
| Signature | 65 B | 3 309 B |
| Public key | — (recovered) | 1 952 B |
| Total size | ~110 B | ~5 314 B |

---

## Gas & Performance

Full analysis in [GAS_COST_ANALYSIS.md](GAS_COST_ANALYSIS.md).

### Summary

| Parameter | Classical | PQ | Note |
|---|---|---|---|
| Simple transfer gas | ~21 976 | ~42 892 | +95% |
| Precompile verify gas | 3 000 | 3 450 | +15% |
| Max transfers/block | ~1 638 | ~839 | 51% capacity |
| Block bandwidth | ~100 KB | ~4.5 MB | 45x |
| Daily storage | ~2.9 GB | ~130 GB | 45x |

The computational overhead of PQ operations is marginal. The dominant cost is
**spatial** — larger keys and signatures increase bandwidth, storage, and reduce
block capacity.

---

## Known Limitations

### 1. Signature Size Bottleneck

The 87x increase in transaction size (5.3 KB vs 61 B) is the primary constraint:

- **Block throughput:** ~50% fewer transactions per block under the same gas limit.
- **P2P bandwidth:** Blocks are ~45x larger, increasing propagation latency.
- **Mempool memory:** Each pending transaction consumes ~50x more RAM.
- **Storage growth:** ~130 GB/day at 1 block per 3 seconds vs ~2.9 GB classically.

This is an inherent property of current lattice-based signature schemes and not
specific to this implementation. It is the principal trade-off accepted in
exchange for quantum resistance.

### 2. No Signature Aggregation

Classical Ethereum PoS uses BLS12-381 signature aggregation — hundreds of
validator signatures collapse into a single 96-byte aggregate. No equivalent
aggregation scheme for ML-DSA-65 exists at production maturity today.

### 3. Non-Recoverable Signatures

ECDSA allows public key recovery from `(r, s, v)`. ML-DSA-65 does not. The
public key must be transmitted with every transaction, contributing 1 952 B of
overhead per tx that cannot be eliminated at the protocol level.

### 4. No Backward Compatibility

Classic ECDSA transactions are not supported. This is intentional — maintaining
a hybrid mode would significantly complicate the design and leave a classical
attack surface open. The trade-off is an explicit design decision.

---

## Considered Alternatives

Several approaches were evaluated to mitigate the signature size bottleneck.
None were adopted for the reasons stated below.

### zkSNARKs per Transaction (Groth16 / PLONK)

**Idea:** Generate a zero-knowledge proof that a valid ML-DSA-65 signature
exists, transmit only the ~200 B proof instead of the 3 309 B signature.

**Why not adopted:**
- Groth16 and PLONK rely on elliptic-curve pairings (BN254, BLS12-381), which
  are broken by Shor's algorithm. This would introduce a quantum-vulnerable
  component into a system designed to be quantum-safe end-to-end.
- Constructing a ZK circuit for ML-DSA-65 internals (lattice arithmetic over
  $\mathbb{Z}_q$, $q = 8\,380\,417$) is an open research problem with no
  production-ready implementation as of 2026.

### ZK Rollup over PQ L1 (Groth16 / PLONK batch proof)

**Idea:** Run a Layer 2 rollup that batches PQ transactions and posts a single
ZK proof to the PQ L1, amortizing signature overhead across thousands of txs.

**Why not adopted:**
- Same issue: the batch proof itself uses elliptic-curve cryptography and is
  not quantum-safe. The L1 would be quantum-resistant while the proof system
  bridging L2→L1 would not be.

### STARKs (quantum-safe ZK)

**Idea:** STARKs rely only on collision-resistant hash functions — no elliptic
curves, quantum-safe by construction. A STARK-based rollup over PQ-EVM would be
genuinely quantum-safe end-to-end.

**Why not adopted:**
- STARK proof sizes are significantly larger than SNARKs (~45–200 KB vs ~200 B),
  partially offsetting the bandwidth savings.
- Recursive STARKs for arbitrary EVM execution (comparable to SNARK-based ZK-EVMs)
  are not yet at production maturity as of 2026.
- Outside the scope of this research project. Identified as the correct long-term
  scaling path once the ecosystem matures.

### Optimistic Rollup over PQ L1

**Idea:** An optimistic rollup uses fraud proofs (hash-only, no ZK) and is
therefore quantum-safe. Users submit PQ txs to L2; only batch roots are posted
to the PQ L1.

**Why not adopted:**
- Introduces a 7-day withdrawal delay (standard optimistic challenge window).
- Requires a separate sequencer and dispute resolution infrastructure outside
  the scope of this project.
- Identified as a viable and quantum-safe scaling path for future work.

### FALCON Signature Aggregation

**Idea:** FALCON (another NIST PQ signature scheme) has structural properties
closer to BLS that may allow limited signature aggregation, reducing the per-tx
signature contribution.

**Why not adopted:**
- FALCON's aggregation properties are still an active research area — no
  standardized aggregation scheme exists.
- FALCON key generation has variable-time implementation risks (timing side
  channels) that require careful mitigation.
- ML-DSA-65 was chosen over FALCON due to its simpler, constant-time
  implementation and final NIST FIPS 204 standardization.

---

## Documentation Index

| Document | Description |
|---|---|
| [CONSENSUS.md](CONSENSUS.md) | Ethereum PoS architecture and the migration to PQ PoA |
| [GAS_COST_ANALYSIS.md](GAS_COST_ANALYSIS.md) | Benchmark results, gas pricing recommendations, throughput analysis |
| [NODE.md](NODE.md) | Node architecture, transaction format, RPC, running instructions |
| [WALLET.md](WALLET.md) | Key generation, address derivation, transaction signing |
| [pq-reth/README_PQ.md](../pq-reth/README_PQ.md) | Detailed crate-level documentation for the reth fork |
