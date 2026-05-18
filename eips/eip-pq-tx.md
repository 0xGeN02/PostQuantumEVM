---
eip: XXXX
title: Post-Quantum Transaction Type with ML-DSA-65
description: Introduces tx type 0x50 using FIPS-204 ML-DSA-65 signatures and SHAKE-256 address derivation for quantum-resistant Ethereum transactions.
author: 0xGeN02 (@0xGeN02)
discussions-to: https://ethereum-magicians.org/t/eip-xxxx-post-quantum-transaction-type/XXXXX
status: Draft
type: Standards Track
category: Core
created: 2026-05-17
requires: 2718
---

## Abstract

This EIP introduces a new [EIP-2718](./eip-2718.md) transaction type (`0x50`) that replaces ECDSA/secp256k1 signatures with FIPS-204 ML-DSA-65 (CRYSTALS-Dilithium) for transaction authentication. It defines a SHAKE-256 based address derivation scheme, a new EVM opcode `PQHASH` (`0x21`) for in-contract SHAKE-256 hashing, and a precompile at address `0x0100` for ML-DSA-65 signature verification. The proposal provides a migration path toward quantum-resistant Ethereum by enabling post-quantum (PQ) accounts to coexist alongside classical accounts during a transition period.

## Motivation

Shor's algorithm, executable on a sufficiently large cryptographically relevant quantum computer (CRQC), can break the discrete-logarithm and integer-factorization assumptions underlying secp256k1 ECDSA in polynomial time. NIST estimates such machines may exist within 10-15 years, and the "harvest now, decrypt later" threat model makes proactive migration essential.

In August 2024, NIST finalized FIPS-204 (ML-DSA), the first standardized post-quantum digital signature scheme. ML-DSA-65 (security level III, ~143-bit classical / ~128-bit quantum) provides a conservative security margin suitable for blockchain transaction signing.

The current Ethereum signature scheme is deeply embedded at the protocol level:

1. **ECDSA recovery** (`ecrecover`) derives the sender address from the signature, making the public key implicit. ML-DSA-65 signatures are **not recoverable** -- the public key must be transmitted explicitly.
2. **Address derivation** uses `keccak256(pubkey_uncompressed[1..])[12..]`, which is secp256k1-specific (65-byte uncompressed keys). ML-DSA-65 public keys are 1952 bytes.
3. **EVM precompiles** (`ecrecover`, BN254, BLS12-381, KZG) rely on classical assumptions that a CRQC would break.

A new transaction type is the cleanest approach: it does not break existing transactions, enables opt-in migration, and allows both classical and PQ transactions to coexist on the same chain.

### Relationship to EIP-7932 and EIP-8051

[EIP-7932](./eip-7932.md) (Secondary Signature Algorithms) proposes a `sigrecover` precompile and algorithm registry. [EIP-8051](./eip-8051.md) proposes ML-DSA verification precompiles. This EIP is complementary: it provides the **native transaction-level** integration that EIP-7932/8051 do not address. Smart contracts on a chain implementing this EIP can use the `0x0100` precompile or `PQHASH` opcode for on-chain PQ verification, while the transaction layer handles protocol-level authentication natively.

## Specification

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in RFC 2119 and RFC 8174.

### Parameters

| Constant | Value | Description |
|----------|-------|-------------|
| `PQ_TX_TYPE` | `0x50` | EIP-2718 transaction type identifier |
| `MLDSA65_SIG_LEN` | `3309` | ML-DSA-65 signature length (bytes) |
| `MLDSA65_PK_LEN` | `1952` | ML-DSA-65 verifying key length (bytes) |
| `MLDSA65_SEED_LEN` | `32` | ML-DSA-65 key generation seed (bytes) |
| `SHAKE256_OUTPUT_LEN` | `32` | SHAKE-256 XOF output for hashing (bytes) |
| `PQHASH_OPCODE` | `0x21` | EVM opcode for SHAKE-256 |
| `MLDSA_PRECOMPILE_ADDR` | `0x0000...0100` | ML-DSA-65 verify precompile address |
| `MLDSA_VERIFY_GAS` | `3450` | Gas cost for ML-DSA-65 verification precompile |

### Transaction Format

After `PQ_TX_TYPE`, the EIP-2718 payload is an RLP-encoded list:

```
0x50 || rlp([chain_id, nonce, gas_price, gas_limit, to, value, input, signature, public_key])
```

| Field Index | Field | Type | Description |
|-------------|-------|------|-------------|
| 0 | `chain_id` | `uint64` | Mandatory replay protection (per [EIP-155](./eip-155.md)) |
| 1 | `nonce` | `uint64` | Sender nonce |
| 2 | `gas_price` | `uint128` | Gas price in wei |
| 3 | `gas_limit` | `uint64` | Maximum gas units |
| 4 | `to` | `bytes` | 20-byte address (call) or empty (contract creation) |
| 5 | `value` | `uint256` | Transfer value in wei |
| 6 | `input` | `bytes` | Calldata or init code |
| 7 | `signature` | `bytes` | ML-DSA-65 signature (exactly 3309 bytes) |
| 8 | `public_key` | `bytes` | ML-DSA-65 verifying key (exactly 1952 bytes) |

**Validation rules:**

- `chain_id` MUST match the node's expected chain ID.
- `gas_limit` MUST be greater than zero.
- `gas_price` MUST be greater than zero.
- `signature` MUST be exactly `MLDSA65_SIG_LEN` bytes.
- `public_key` MUST be exactly `MLDSA65_PK_LEN` bytes.
- `to` MUST be either empty (contract creation) or exactly 20 bytes (call).

### Signing Hash

The message to be signed is computed deterministically from the transaction fields using SHAKE-256:

```
signing_hash = SHAKE-256(
    PQ_TX_TYPE              ||  // 1 byte
    chain_id (big-endian)   ||  // 8 bytes
    nonce (big-endian)      ||  // 8 bytes
    gas_price (big-endian)  ||  // 16 bytes
    gas_limit (big-endian)  ||  // 8 bytes
    to_flag                 ||  // 1 byte: 0x01 if to is present, 0x00 otherwise
    [to_address]            ||  // 20 bytes (only if to_flag == 0x01)
    value (big-endian)      ||  // 16 bytes
    input                       // variable length
, output_length = 32)
```

The ML-DSA-65 signing key then signs the 32-byte `signing_hash`:

```
signature = ML-DSA-65.Sign(signing_key, signing_hash)
```

### Transaction Hash

The transaction hash uniquely identifies the signed transaction:

```
tx_hash = SHAKE-256(
    PQ_TX_TYPE      ||  // 1 byte
    signing_hash    ||  // 32 bytes
    signature       ||  // 3309 bytes
    public_key          // 1952 bytes
, output_length = 32)
```

### Address Derivation

PQ addresses are derived from the ML-DSA-65 public key using SHAKE-256:

```
address = SHAKE-256(public_key, output_length = 32)[12..32]
```

This takes the last 20 bytes of the 32-byte SHAKE-256 output, mirroring the classical Ethereum pattern `keccak256(pk)[12..]` but using a quantum-safe hash.

### Sender Recovery

ML-DSA-65 signatures are not recoverable. The sender address MUST be derived from the `public_key` field embedded in the transaction:

1. Verify `ML-DSA-65.Verify(public_key, signing_hash, signature)` returns `true`.
2. Compute `sender = SHAKE-256(public_key, 32)[12..32]`.
3. If verification fails, the transaction MUST be rejected.

### Receipt Format

Type `0x50` receipts follow [EIP-2718](./eip-2718.md):

```
0x50 || rlp([status, cumulative_gas_used, logs_bloom, logs])
```

No changes to the receipt structure relative to existing transaction types.

### `PQHASH` Opcode (`0x21`)

A new EVM opcode is introduced adjacent to `KECCAK256` (`0x20`):

- **Opcode:** `0x21`
- **Mnemonic:** `PQHASH`
- **Stack input:** `offset, length`
- **Stack output:** `hash`
- **Gas:** `30 + 6 * ceil(length / 32)` (same formula as `KECCAK256`) plus memory expansion cost
- **Semantics:** Computes `SHAKE-256(memory[offset..offset+length], output_length = 32)`

This opcode allows smart contracts to compute PQ addresses and verify PQ-related data structures without calling an external precompile.

### ML-DSA-65 Verification Precompile (`0x0100`)

A new precompile is deployed at address `0x0000000000000000000000000000000000000100`:

- **Input:** `msg_hash (32 bytes) || signature (3309 bytes) || public_key (1952 bytes)` = 5293 bytes total
- **Output:** `uint256(1)` if valid, `uint256(0)` if invalid (32 bytes, ABI-encoded)
- **Gas cost:** `MLDSA_VERIFY_GAS` = 3450 (static)
- **Error:** If input length != 5293 bytes, the precompile MUST return an error (consume all gas).

The gas cost is calibrated from benchmarks: ML-DSA-65 verification takes ~42 microseconds on modern hardware, at ~61 gas/microsecond with a 1.35x safety margin.

### Disabled Classical Precompiles

The following precompiles MUST return an error (consume all gas) when invoked on chains that exclusively support PQ transactions:

| Address | Name | Reason |
|---------|------|--------|
| `0x01` | ecrecover | ECDSA vulnerable to Shor's algorithm |
| `0x06` | ecAdd | BN254 - pairing-based, vulnerable |
| `0x07` | ecMul | BN254 - pairing-based, vulnerable |
| `0x08` | ecPairing | BN254 - pairing-based, vulnerable |
| `0x0a` | KZG point eval | Elliptic-curve based, vulnerable |
| `0x0b`-`0x13` | BLS12-381 (9 ops) | Pairing-based, vulnerable |

The following quantum-safe precompiles MUST remain active:

| Address | Name | Reason |
|---------|------|--------|
| `0x02` | SHA-256 | Hash function, quantum-safe (Grover halves, still >=128-bit) |
| `0x03` | RIPEMD-160 | Hash function, quantum-safe |
| `0x04` | Identity | Data copy, no crypto |
| `0x05` | ModExp | Modular exponentiation, useful for RSA legacy interop |
| `0x09` | Blake2f | Hash function, quantum-safe |

> **Note:** On hybrid chains (supporting both classical and PQ transactions during a transition period), the classical precompiles MAY remain active. The disabling applies only to PQ-native chains.

## Rationale

### Why a new transaction type?

Account abstraction (ERC-4337) combined with EIP-7702 could theoretically enable PQ signatures at the application layer. However, this approach has fundamental limitations:

1. **Gas overhead:** Each UserOperation incurs ~20,000+ gas overhead for the EntryPoint contract. Native PQ transactions avoid this.
2. **No protocol-level security:** Application-layer PQ signatures do not protect the consensus layer. A CRQC could forge validator signatures.
3. **Wallet complexity:** Every wallet would need to implement PQ signature construction independently. A native tx type standardizes the format.

### Why ML-DSA-65?

| Parameter | ML-DSA-44 | **ML-DSA-65** | ML-DSA-87 |
|-----------|-----------|---------------|-----------|
| Security level | II (128-bit) | **III (192-bit)** | V (256-bit) |
| Public key | 1312 B | **1952 B** | 2592 B |
| Signature | 2420 B | **3309 B** | 4627 B |
| Sign time | ~30 us | ~50 us | ~80 us |
| Verify time | ~30 us | ~42 us | ~62 us |

ML-DSA-65 provides a pragmatic balance: NIST security level III offers a conservative margin (192-bit classical, 128-bit quantum) while keeping signature and key sizes manageable. Level V (ML-DSA-87) adds 40% size for marginal quantum security improvement.

### Why SHAKE-256 for address derivation?

1. **Consistency:** ML-DSA-65 internally uses SHAKE-256. Using the same primitive for address derivation simplifies the cryptographic dependency tree.
2. **Quantum safety:** SHAKE-256 is a member of the SHA-3 family (Keccak-based), with 256-bit pre-image resistance under quantum (Grover's algorithm reduces to 128-bit, still sufficient).
3. **Collision avoidance:** `SHAKE-256(pk)` produces different outputs than `keccak256(pk)` for any input. Combined with the different public key format (1952 bytes vs 64 bytes), address collision between PQ and classical accounts is computationally infeasible.

### Why tx type `0x50`?

- Types `0x00`-`0x04` are allocated. Types `0x05`-`0x4f` are available for sequential allocation by future EIPs.
- `0x50` (ASCII `'P'`) is mnemonic for "Post-quantum" and avoids occupying the sequential allocation space.
- Using a higher number signals that this is a domain-specific extension rather than a core protocol evolution.

### Why flat gas price instead of EIP-1559?

The current implementation uses a flat `gas_price` field for simplicity. A production deployment SHOULD adopt EIP-1559-style fee fields (`max_fee_per_gas`, `max_priority_fee_per_gas`) by extending the RLP list. This EIP specifies the minimal viable format; a follow-up EIP can add EIP-1559 fee market support.

### Why include the public key in every transaction?

Unlike ECDSA, ML-DSA signatures do not support public key recovery. The signer's public key MUST be transmitted alongside the signature. At 1952 bytes per transaction, this increases bandwidth by ~30x compared to ECDSA transactions. Mitigation strategies for future optimization:

1. **Public key registry:** A mapping `address -> public_key` stored on-chain. After the first transaction, subsequent transactions could reference the registry instead of embedding the key. This requires a separate EIP.
2. **Compression:** ML-DSA public keys have internal structure that could be compressed. NIST is evaluating compression standards.
3. **Aggregation:** Batch signatures for multiple transactions from the same sender could share the public key. This requires sequencer-level support.

## Backwards Compatibility

### Non-breaking for existing transactions

Type `0x50` transactions are additive under [EIP-2718](./eip-2718.md). Nodes that do not implement this EIP MUST reject transactions with `TransactionType = 0x50` as unknown, which is the specified behavior in EIP-2718. Existing transaction types (`0x00`-`0x04`) are unaffected.

### Address space

PQ addresses (`SHAKE-256(pk)[12..]`) and classical addresses (`keccak256(pk)[12..]`) share the same 20-byte format but are derived from different hash functions and different key types. The probability of collision is `1/2^160`, which is negligible.

### Broken invariants

1. **Sender recovery from signature alone is no longer possible.** Tools that derive the sender by calling `ecrecover` on the v/r/s fields MUST be updated to extract the `public_key` field and compute `SHAKE-256(pk)[12..]` for type `0x50` transactions.
2. **Transaction size increases significantly.** A minimal PQ transfer is ~5.3 KB vs ~110 bytes for ECDSA. Block builders, mempool implementations, and P2P gossip protocols MUST account for this.
3. **The `from` field in RPC responses** MUST be derived from the embedded public key, not from signature recovery.

### EVM compatibility

Contracts that call `ecrecover` (`0x01`) will get an error on PQ-native chains. Contracts SHOULD use the `PQHASH` opcode (`0x21`) or the ML-DSA-65 precompile (`0x0100`) for PQ-compatible signature verification.

## Test Cases

### 1. Transaction encoding and decoding

Given a transaction with:
- `chain_id = 20561`
- `nonce = 0`
- `gas_price = 1000000000` (1 gwei)
- `gas_limit = 21000`
- `to = 0x4d0E5AF04B8Ce167de49f4b5E38fFa31b3e74fBe`
- `value = 1000000000000000000` (1 qETH)
- `input = 0x` (empty)

The signing hash MUST be computed as:
```
signing_hash = SHAKE-256(
    0x50 ||
    0x0000000000005051 ||  // chain_id 20561
    0x0000000000000000 ||  // nonce 0
    0x00000000000000000000003B9ACA00 ||  // gas_price 1e9
    0x0000000000005208 ||  // gas_limit 21000
    0x01 || 0x4d0E5AF04B8Ce167de49f4b5E38fFa31b3e74fBe ||  // to
    0x00000000000000000DE0B6B3A7640000 ||  // value 1e18
    (empty)
, 32)
```

The transaction MUST be serializable as `0x50 || rlp([...])` and deserializable back to the same fields.

### 2. Signature verification

A valid ML-DSA-65 signature over the signing hash MUST pass verification. A signature with any byte modified MUST fail.

### 3. Address derivation

The address derived from a known public key via `SHAKE-256(pk, 32)[12..32]` MUST match the expected 20-byte address.

### 4. Multi-node consistency

Three validators running PoA consensus with PQ transactions MUST maintain consistent chain state:
- Same chain ID across all nodes
- Block height drift <= 2 blocks
- Identical account balances for genesis-funded accounts

### 5. Smart contract interaction

- A PQ-signed transaction MUST be able to deploy a contract.
- `eth_call` MUST work for reading contract state via PQ-signed read calls.
- The `PQHASH` opcode MUST return the same result as off-chain SHAKE-256 computation.

## Reference Implementation

A complete reference implementation is available at:

- **Execution client (reth fork):** [github.com/0xGeN02/pq-reth](https://github.com/0xGeN02/pq-reth) -- reth modified for ML-DSA-65 transaction signing, PQ consensus validation, `PQHASH` opcode, and ML-DSA-65 precompile.
- **Wallet & tooling:** [github.com/0xGeN02/PostQuantumEVM](https://github.com/0xGeN02/PostQuantumEVM) -- ML-DSA-65 keystore, CLI wallet, TUI wallet, and E2E test suite.
- **Cryptographic library:** [ml-lattice-rs](https://github.com/0xGeN02/ml-lattice-rs) -- Rust implementation of ML-DSA (FIPS-204).

The implementation passes a full E2E validation suite:
- 7 readonly chain validation tests
- 5 PQ transaction tests (transfer, receipt, nonce, contract deploy, contract call)
- 3 multi-node consistency tests (chain ID, block height, state)

## Security Considerations

### Quantum threat timeline

The EIP is motivated by the projected 10-15 year timeline for CRQC development. However, the "harvest now, decrypt later" attack is already relevant: an adversary can record signed transactions today and forge signatures once a CRQC is available. Early adoption of PQ signatures protects against this retroactive threat.

### ML-DSA-65 security level

ML-DSA-65 provides NIST security level III (192-bit classical, 128-bit quantum security against key recovery). This is the RECOMMENDED level for blockchain applications. Level II (ML-DSA-44) provides only 128-bit classical security, which is below the current secp256k1 security margin.

### Signature malleability

ML-DSA-65 signatures are deterministic (no random nonce). Given the same signing key and message, `Sign` always produces the same signature. This eliminates signature malleability, which was a historical concern with ECDSA (low-s normalization, EIP-2).

### Public key exposure

Including the full 1952-byte public key in every transaction exposes it on-chain. For ML-DSA-65, this is not a security concern: the hardness assumption (Module-LWE) ensures that recovering the signing key from the public key is computationally infeasible even with a quantum computer.

### Transaction size and DoS

A minimal PQ transaction (~5.3 KB) is ~48x larger than a minimal ECDSA transaction (~110 bytes). This increases:
- **Mempool memory:** Nodes SHOULD enforce per-transaction size limits. A 128 KB cap accommodates PQ transactions with large calldata.
- **P2P bandwidth:** Block propagation time increases proportionally. For a 30M gas block, the theoretical maximum PQ transaction count is lower, providing natural back-pressure.
- **Calldata gas:** At 16 gas per non-zero byte, the PQ overhead is `(3309 + 1952) * 16 = 84,176` gas per transaction. Combined with the 21,000 base cost, a minimal PQ transfer costs ~105,176 gas.

Implementations SHOULD consider adjusting the block gas limit or introducing a separate "PQ gas" dimension to prevent PQ transactions from crowding out classical transactions during a transition period.

### Address collision between classical and PQ accounts

The probability that `SHAKE-256(mldsa65_pk)[12..32]` collides with `keccak256(secp256k1_pk)[12..32]` for any key pair is `1/2^160`. Additionally, the domain separation (different hash functions, different key formats) provides defense in depth. This is considered cryptographically negligible.

### Side-channel resistance

ML-DSA-65 implementations MUST be constant-time to prevent timing side-channel attacks. The reference implementation uses the `ml-lattice-rs` library which implements constant-time arithmetic for all operations involving secret key material.

### Transition period risks

During a hybrid period where both classical and PQ transactions coexist:
1. An attacker with a CRQC could forge classical transactions but not PQ transactions. Users SHOULD migrate to PQ accounts before a CRQC becomes available.
2. Smart contracts that verify signatures on-chain MUST be updated to use the ML-DSA-65 precompile (`0x0100`) or `PQHASH` opcode instead of `ecrecover`.
3. Chain consensus (PoS validators) remains ECDSA/BLS12-381 based until a separate consensus-layer migration is completed.

## Copyright

Copyright and related rights waived via [CC0](../LICENSE.md).
