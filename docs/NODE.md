# PostQuantumEVM Node Documentation

## Overview

PostQuantumEVM is a post-quantum resistant Ethereum execution client built as a non-invasive extension of [reth](https://github.com/paradigmxyz/reth). It replaces ECDSA/secp256k1 entirely with **ML-DSA-65 (CRYSTALS-Dilithium)** for transaction signing and verification, adds a native **SHAKE-256** hashing opcode, and disables all classical elliptic curve precompiles vulnerable to quantum attacks.

The project follows NIST FIPS 204 (ML-DSA) and FIPS 203 (ML-KEM) standards.

---

## Architecture

```
PostQuantumEVM/
├── pq-reth/                         # Forked reth (git submodule)
│   └── crates/pq/                   # 7 new PQ crates (non-invasive)
│       ├── reth-pq-primitives       # Core types: PqSignedTransaction, PqSigner
│       ├── reth-pq-consensus        # Transaction validation (ML-DSA-65 verify)
│       ├── reth-pq-precompile       # ML-DSA-65 verify precompile at 0x0100
│       ├── reth-pq-evm             # Custom EvmFactory, disabled precompiles
│       ├── reth-pq-pool            # Transaction pool with PQ validation
│       ├── reth-pq-node-primitives  # PqPrimitives (NodePrimitives impl)
│       └── reth-pq-node            # PqNode definition, RPC, engine, payload
├── ml-lattice-rs/                   # PQ crypto library (submodule)
│   ├── dilithium/                   # ML-DSA wrapper (FIPS 204)
│   └── kyber/                       # ML-KEM wrapper (FIPS 203)
├── pq-wallet/                       # PQ wallet (CLI + core)
└── qiskit-api/                      # Quantum attack simulation (Shor/Grover)
```

### Integration Strategy

All PQ code lives inside the reth workspace as new crates under `crates/pq/`. No upstream reth code is modified except for an optional `pq` feature flag in `reth-rpc-convert`. This enables clean rebasing against upstream reth updates.

---

## Transaction Format

### Type Envelope

PQ transactions use EIP-2718 type **`0x04`**.

### Unsigned Transaction Fields

| Field | Type | Description |
|-------|------|-------------|
| `chain_id` | `u64` | Chain identifier |
| `nonce` | `u64` | Sender's transaction count |
| `gas_price` | `u128` | Gas price in wei |
| `gas_limit` | `u64` | Maximum gas for execution |
| `to` | `Option<Address>` | Recipient (None for contract creation) |
| `value` | `u128` | ETH value in wei |
| `input` | `Bytes` | Calldata / contract init code |

### Signing Hash

```
signing_hash = keccak256(
    0x04 ||
    chain_id (8 bytes BE) ||
    nonce (8 bytes BE) ||
    gas_price (16 bytes BE) ||
    gas_limit (8 bytes BE) ||
    to_flag (1 byte: 0x01 if Some, 0x00 if None) ||
    [to (20 bytes) if Some] ||
    value (16 bytes BE) ||
    input (variable)
)
```

> **Note**: Will be migrated to SHAKE-256 in a future update for cryptographic consistency with ML-DSA.

### Signed Transaction

A signed PQ transaction contains:
- The unsigned transaction fields
- ML-DSA-65 signature (3309 bytes)
- ML-DSA-65 public key (1952 bytes)
- Cached transaction hash

### Transaction Hash

```
tx_hash = keccak256(0x04 || signing_hash || signature_bytes || public_key_bytes)
```

### Wire Format (EIP-2718 encoded)

```
0x04 || RLP([chain_id, nonce, gas_price, gas_limit, to, value, input, signature, public_key])
```

### Sender Recovery

Unlike ECDSA, ML-DSA signatures are **not recoverable** — the public key is embedded directly in the transaction. The sender address is derived as:

```
sender = keccak256(public_key_bytes)[12..]
```

---

## EVM Changes

### New Opcode

| Opcode | Name | Stack | Description | Gas |
|--------|------|-------|-------------|-----|
| `0x21` | **PQHASH** | `(offset, length) → hash` | Computes SHAKE-256 over memory region, returns 32-byte digest | 30 + 6 per word |

**Rationale**: ML-DSA internally uses SHAKE-256. Adding a native opcode ensures cryptographic consistency — the entire PQ layer (signatures + hashing) uses the same SHA-3 family. The slot `0x21` is adjacent to KECCAK256 (`0x20`).

Smart contracts can use `PQHASH` for post-quantum secure hashing while `KECCAK256` remains available for backward compatibility.

### Disabled Precompiles (13)

All classical elliptic curve precompiles are disabled and return `PrecompileError`:

| Address | Name | Reason |
|---------|------|--------|
| `0x01` | ecrecover | ECDSA — broken by Shor's algorithm |
| `0x06` | ecAdd | BN254 point addition — classical curve |
| `0x07` | ecMul | BN254 scalar multiplication — classical curve |
| `0x08` | ecPairing | BN254 pairing — broken by Shor's |
| `0x0a` | point_evaluation | KZG (EIP-4844) — relies on BLS12-381 DLP |
| `0x0b` | bls12_g1Add | BLS12-381 — classical curve |
| `0x0c` | bls12_g1Mul | BLS12-381 — classical curve |
| `0x0d` | bls12_g1Msm | BLS12-381 — classical curve |
| `0x0e` | bls12_g2Add | BLS12-381 — classical curve |
| `0x0f` | bls12_g2Mul | BLS12-381 — classical curve |
| `0x10` | bls12_g2Msm | BLS12-381 — classical curve |
| `0x11` | bls12_pairing | BLS12-381 — classical curve |
| `0x12` | bls12_map_fp_to_g1 | Hash-to-curve on broken curve |
| `0x13` | bls12_map_fp2_to_g2 | Hash-to-curve on broken curve |

### Kept Precompiles (quantum-safe)

| Address | Name | Justification |
|---------|------|---------------|
| `0x02` | SHA-256 | Hash function — Grover reduces to 128-bit (sufficient) |
| `0x03` | RIPEMD-160 | Hash function |
| `0x04` | Identity | Data copy — no cryptographic assumptions |
| `0x05` | ModExp | Pure arithmetic — not a security primitive |
| `0x09` | Blake2f | Hash compression — quantum-safe |

### New Precompiles

| Address | Name | Input | Output | Gas |
|---------|------|-------|--------|-----|
| `0x0100` | **pq_verify** | `msg_hash(32) \|\| sig(3309) \|\| pk(1952)` = 5293 bytes | `0x01` (valid) or `0x00` (invalid) | 50,000 (pending benchmark) |
| `0x0101` | **pq_batch_verify** | `N(4) \|\| [msg_hash(32) \|\| sig(3309) \|\| pk(1952)] × N` | `0x01` (all valid) or `0x00` | ~40,000 × N × 0.7 |
| `0x0102` | **pq_decapsulate** | `ciphertext(1088) \|\| dk(2400)` = 3488 bytes | `shared_secret(32)` | TBD |

### Unchanged EVM Opcodes

All standard EVM opcodes remain functional:

- **`CALLER` / `ORIGIN`** — Work normally. Addresses are derived from ML-DSA public keys but remain 20 bytes.
- **`CREATE` / `CREATE2`** — Unchanged. Contract address derivation uses keccak256 over sender address + nonce/salt (no public key involved).
- **`KECCAK256` (0x20)** — Kept for internal EVM operations (storage slots, ABI encoding, function selectors).
- **`SELFDESTRUCT`** — Unchanged (deprecated per Ethereum roadmap).

---

## Consensus

### Block-Level Consensus

The PQ node uses **`EthBeaconConsensus`** unchanged — standard Ethereum PoS block validation rules (gas limits, timestamps, difficulty).

### Transaction-Level Validation

`PqTransactionValidator` performs:
1. Chain ID verification
2. Gas limit > 0 check
3. **ML-DSA-65 signature verification** (replaces ECDSA recovery)

### Demo Mode

For testing/demo purposes, reth's built-in `--dev` mode is used:
- Auto-mines blocks at a configurable interval
- No external Consensus Layer client required
- Block production handled internally

---

## Node Components

| Component | Implementation | Purpose |
|-----------|---------------|---------|
| `PqNode` | `NodeTypes` + `Node<N>` | Top-level node type definition |
| `PqEvmConfig` | `ConfigureEvm` | EVM with PQ precompiles + PQHASH opcode |
| `PqEvmFactory` | `EvmFactory` | Creates EVM instances with custom precompile set |
| `PqPoolValidator` | `TransactionValidator` | Mempool ML-DSA-65 signature check |
| `PqConsensusBuilder` | `EthBeaconConsensus` | Block consensus rules |
| `PqEngineValidator` | Engine API handler | Payload validation for CL communication |
| `PqPayloadBuilder` | Payload construction | Pulls PQ txs from pool, executes via PqEvmConfig |
| `PqRpcTxConverter` | RPC layer | Converts PQ transactions for JSON-RPC responses |
| `PqReceiptBuilder` | Receipt construction | Maps PQ tx type to receipt format |

---

## Cryptographic Primitives

| Primitive | Algorithm | Standard | Size |
|-----------|-----------|----------|------|
| Signing key | ML-DSA-65 | NIST FIPS 204 | 4032 bytes |
| Verifying key | ML-DSA-65 | NIST FIPS 204 | 1952 bytes |
| Signature | ML-DSA-65 | NIST FIPS 204 | 3309 bytes |
| Key encapsulation | ML-KEM-768 | NIST FIPS 203 | 1184 / 2400 / 1088 bytes |
| Protocol hashing | SHAKE-256 | NIST FIPS 202 | 32 bytes output |
| EVM hashing (opcode) | SHAKE-256 | NIST FIPS 202 | 32 bytes output |
| EVM hashing (legacy) | KECCAK-256 | — | 32 bytes output |

---

## Network Configuration

### Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 30303 | TCP+UDP | P2P (devp2p RLPx + discovery) |
| 8545 | TCP | HTTP JSON-RPC |
| 8546 | TCP | WebSocket JSON-RPC |
| 9001 | TCP | Metrics (Prometheus) |
| 8551 | TCP | Engine API (if using CL) |

### P2P

Standard Ethereum P2P stack is reused (devp2p). PQ transactions are ~5.3KB (vs ~100-200 bytes for classical ECDSA transactions) due to larger signatures and embedded public keys.

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

The following patterns **no longer work** on the PQ chain:

| Pattern | Status | Alternative |
|---------|--------|-------------|
| `ecrecover(hash, v, r, s)` | Reverts always | Use `PQVerify.verify()` |
| OpenZeppelin `ECDSA.recover()` | Broken | Use PQ signature checker |
| ERC-2612 `permit()` | Broken | Implement PQ-based permit |
| EIP-712 typed data signing | Broken | Use ML-DSA over typed hash |
| BLS signature aggregation | Broken | Not available (ML-DSA has no native aggregation) |
| ZK proof verification (Groth16) | Broken | Requires PQ-friendly ZK system |

### Using PQHASH in Solidity

The `PQHASH` opcode (`0x21`) is accessible via inline assembly:

```solidity
function pqHash(bytes memory data) internal pure returns (bytes32 result) {
    assembly {
        result := pqhash(add(data, 0x20), mload(data))
    }
}
```

> **Note**: Solidity compiler support for the `pqhash` mnemonic requires a custom build or inline `verbatim` usage until upstream support is added.
