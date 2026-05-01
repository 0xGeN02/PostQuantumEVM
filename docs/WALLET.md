# PostQuantumEVM Wallet & Account Documentation

## Overview

The PQ wallet generates and manages **ML-DSA-65 (CRYSTALS-Dilithium)** keypairs for signing transactions on the PostQuantumEVM blockchain. It replaces the classical ECDSA/secp256k1 key management entirely.

---

## Key Generation

### Algorithm

**ML-DSA-65** (NIST FIPS 204, Security Level 3)

| Component | Size | Description |
|-----------|------|-------------|
| Seed | 32 bytes | Random entropy (source of all key material) |
| Signing key | 4032 bytes | Private key for producing signatures |
| Verifying key | 1952 bytes | Public key for signature verification |
| Signature | 3309 bytes | Produced per message signed |

### Process

```
1. Generate 32 random bytes (seed) from OS CSPRNG
2. Expand seed → ML-DSA-65 keypair via FIPS 204 key generation
3. Extract verifying key (public key) → 1952 bytes
4. Derive address from public key (see Address Derivation below)
```

### Implementation

```rust
use ml_dsa::{MlDsa65, KeyGen};

// Random keypair
let (signing_key, verifying_key) = dilithium::dilithium65::keygen();

// Deterministic from seed
let signing_key = MlDsa65::from_seed(&seed_32_bytes);
let verifying_key = signing_key.verifying_key();
```

Only the **32-byte seed** is stored. The full signing key (4032 bytes) is deterministically derived from it on load.

---

## Address Derivation

### Formula

```
address = keccak256(verifying_key_bytes)[12..]
```

> **Future**: Will migrate to `shake256(verifying_key_bytes, 32)[12..]` for cryptographic consistency with ML-DSA (which uses SHAKE-256 internally).

### Step by Step

```
1. Serialize the ML-DSA-65 verifying key → 1952 raw bytes
2. Compute hash = keccak256(1952 bytes) → 32 bytes
3. Take the last 20 bytes: hash[12..32]
4. Result: 20-byte Ethereum-compatible address
```

### Example

```
Verifying Key: [1952 bytes of ML-DSA-65 public key]
                    ↓ keccak256
Hash:         0x7a8b...3f4e...9c2d...1a0b (32 bytes)
                              ↓ [12..]
Address:      0x3f4e...9c2d...1a0b (20 bytes)
```

### Comparison with Classical Ethereum

| | Classical Ethereum | PostQuantumEVM |
|---|---|---|
| Key algorithm | secp256k1 (ECDSA) | ML-DSA-65 (Dilithium) |
| Public key size | 64 bytes (uncompressed) | 1952 bytes |
| Hash function | keccak256 | keccak256 (→ SHAKE-256) |
| Input to hash | 64-byte ECDSA public key | 1952-byte ML-DSA-65 verifying key |
| Address size | 20 bytes | 20 bytes |
| Address format | Identical | Identical |

Addresses remain **20 bytes** and are indistinguishable from classical Ethereum addresses. All existing tooling (explorers, wallets, Solidity `address` type) works without modification.

---

## Account Model

### Account Structure

An account in PostQuantumEVM consists of:

| Field | Source | Stored |
|-------|--------|--------|
| **Seed** | 32 random bytes (CSPRNG) | Encrypted in keystore |
| **Signing key** | Derived from seed (4032 bytes) | In memory only |
| **Verifying key** | Derived from signing key (1952 bytes) | Plaintext in keystore |
| **Address** | Derived from verifying key (20 bytes) | Plaintext in keystore |

### Key Hierarchy

```
seed (32 bytes, secret)
  └─→ SigningKey<MlDsa65> (4032 bytes, secret, in-memory only)
        └─→ VerifyingKey<MlDsa65> (1952 bytes, public)
              └─→ Address (20 bytes, public)
```

The seed is the **single secret** that must be protected. Everything else is derivable.

---

## Keystore Format

### Encryption Scheme

| Layer | Algorithm | Parameters |
|-------|-----------|------------|
| **KDF** | Argon2id | m=65536 (64MB), t=3 iterations, p=4 parallelism |
| **Cipher** | AES-256-GCM | Authenticated encryption |
| **Salt** | Random | 16 bytes |
| **Nonce/IV** | Random | 12 bytes |

### What Is Encrypted

Only the **32-byte seed** is encrypted — not the full 4032-byte signing key. This minimizes storage while maintaining full recoverability.

### JSON File Format

```json
{
  "version": 1,
  "address": "0x3f4e9c2d1a0b...",
  "public_key": "abcdef1234...",
  "crypto": {
    "kdf": "argon2id",
    "kdf_params": {
      "m": 65536,
      "t": 3,
      "p": 4,
      "salt": "hex-encoded-16-bytes"
    },
    "cipher": "aes-256-gcm",
    "cipher_params": {
      "iv": "hex-encoded-12-bytes"
    },
    "ciphertext": "hex-encoded-encrypted-seed"
  }
}
```

### Security Properties

- **Argon2id** is the recommended KDF by OWASP and NIST for password hashing (resistant to GPU/ASIC attacks)
- **AES-256-GCM** provides both confidentiality and integrity (authenticated encryption)
- **64MB memory cost** makes brute-force attacks expensive
- The seed (32 bytes) has 256 bits of entropy — unbreakable even by quantum computers (Grover reduces to 128-bit, still sufficient)

### Operations Without Passphrase

The following can be read from the keystore **without decryption**:
- Address (`address` field)
- Public key (`public_key` field)

This allows displaying account info without requiring the passphrase.

---

## Transaction Signing

### Signing Flow

```
1. Construct PqTxRequest (unsigned transaction fields)
2. Compute signing_hash = keccak256(canonical encoding of fields)
3. Sign: signature = ml_dsa_65_sign(signing_key, signing_hash) → 3309 bytes
4. Attach public key (1952 bytes) to the signed transaction
5. Compute tx_hash = keccak256(0x04 || signing_hash || signature || public_key)
```

### Signing Hash Computation

```
signing_hash = keccak256(
    type_byte (0x04)           ||    // 1 byte
    chain_id                   ||    // 8 bytes, big-endian
    nonce                      ||    // 8 bytes, big-endian
    gas_price                  ||    // 16 bytes, big-endian
    gas_limit                  ||    // 8 bytes, big-endian
    to_flag                    ||    // 1 byte (0x01 present, 0x00 absent)
    [to_address]               ||    // 20 bytes (only if to_flag = 0x01)
    value                      ||    // 16 bytes, big-endian
    input                            // variable length
)
```

### Why the Public Key Is Embedded

In ECDSA, the public key can be **recovered** from the signature + message hash (that's what `ecrecover` does). ML-DSA signatures are **not recoverable** — there is no way to derive the signer's public key from the signature alone.

Therefore, PQ transactions must include the full public key (1952 bytes) alongside the signature. The sender address is derived from this embedded key:

```
sender_address = keccak256(embedded_public_key)[12..]
```

### Signature Verification (at node level)

```
1. Extract public_key from transaction
2. Derive expected_address = keccak256(public_key)[12..]
3. Recompute signing_hash from transaction fields
4. Verify: ml_dsa_65_verify(public_key, signing_hash, signature) → bool
5. If valid, sender = expected_address
```

---

## Transaction Size Comparison

| Component | ECDSA (classical) | ML-DSA-65 (PQ) | Increase |
|-----------|-------------------|----------------|----------|
| Signature | 65 bytes (r, s, v) | 3309 bytes | ~51× |
| Public key | 0 bytes (recovered) | 1952 bytes | — |
| Overhead per tx | 65 bytes | 5261 bytes | ~81× |
| Typical tx total | ~100-200 bytes | ~5,400 bytes | ~30-50× |

This is a fundamental tradeoff of post-quantum cryptography: larger keys and signatures in exchange for quantum resistance.

---

## CLI Commands

### Available Commands

| Command | Description | Requires Passphrase |
|---------|-------------|---------------------|
| `pq-wallet new` | Generate new ML-DSA-65 keypair and save encrypted keystore | Yes (to encrypt) |
| `pq-wallet address` | Display address from keystore | No |
| `pq-wallet balance` | Query account balance via RPC | No |
| `pq-wallet send` | Build, sign, and broadcast a PQ transaction | Yes (to sign) |
| `pq-wallet sign` | Sign an arbitrary message | Yes (to sign) |

### Usage Examples

```bash
# Generate a new PQ account
pq-wallet new --keystore ./my-keystore.json

# Show the address
pq-wallet address --keystore ./my-keystore.json

# Check balance
pq-wallet balance --keystore ./my-keystore.json --rpc http://localhost:8545

# Send a transaction
pq-wallet send \
  --keystore ./my-keystore.json \
  --to 0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18 \
  --value 1000000000000000000 \
  --rpc http://localhost:8545

# Dry run (sign but don't broadcast)
pq-wallet send \
  --keystore ./my-keystore.json \
  --to 0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18 \
  --value 1000000000000000000 \
  --dry-run
```

---

## Security Considerations

### Quantum Resistance

| Attack | Classical (ECDSA) | PostQuantumEVM (ML-DSA-65) |
|--------|-------------------|---------------------------|
| Shor's algorithm (ECDLP) | Key recovery from public key | Not applicable (no elliptic curves) |
| Grover's algorithm (hash preimage) | Reduces keccak256 to 128-bit | 128-bit security (sufficient) |
| Lattice reduction (LWE/SIS) | Not applicable | Primary threat — NIST Level 3 security |
| Side-channel attacks | Standard mitigations | Standard mitigations |

### Seed Security

- The 32-byte seed has 256 bits of entropy
- Even with Grover's algorithm, brute-forcing requires 2^128 operations (computationally infeasible)
- Argon2id KDF ensures passphrase-derived key is expensive to brute-force
- AES-256-GCM ensures ciphertext tampering is detected

### Key Size Implications

The large public key (1952 bytes) means:
- Transactions are ~5.3KB vs ~100-200 bytes classical
- Block space is consumed faster per transaction
- Network bandwidth requirements increase
- Storage per account increases

These are accepted tradeoffs for quantum resistance.

---

## Migration Notes (SHAKE-256)

The following changes are planned for address derivation and hashing:

### Before (current)
```
address = keccak256(ml_dsa_65_public_key)[12..]
signing_hash = keccak256(encoded_fields)
tx_hash = keccak256(0x04 || signing_hash || sig || pk)
```

### After (planned)
```
address = shake256(ml_dsa_65_public_key, 32)[12..]
signing_hash = shake256(encoded_fields, 32)
tx_hash = shake256(0x04 || signing_hash || sig || pk, 32)
```

### Rationale

ML-DSA internally uses SHAKE-256 (SHA-3 XOF). Migrating protocol-level hashing to the same primitive ensures:
1. **Cryptographic consistency** — single hash family across the entire PQ stack
2. **Alignment with NIST standards** — SHAKE-256 is part of FIPS 202
3. **No additional dependencies** — already required by the `ml-dsa` crate

The state trie (Merkle Patricia Trie) will **not** be migrated — it continues using keccak256 as modifying it would require rewriting the entire database and proof system.
