# PostQuantumEVM Wallet & Account Documentation

## Overview

The PQ wallet ecosystem provides tools for generating and managing **ML-DSA-65 (CRYSTALS-Dilithium)** keypairs, signing transactions, and interacting with the PostQuantumEVM blockchain. It replaces ECDSA/secp256k1 key management entirely.

### Components

| Binary | Description |
|--------|-------------|
| `pq-wallet` | CLI wallet (keygen, send, sign, balance, call) |
| `pq-tui` | Terminal UI dashboard (4 tabs, interactive actions) |
| `pq-seed` | Chain seeder (pre-populate demo transactions) |
| `pq-e2e` | End-to-end validation test suite |

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
4. Derive address: shake256(verifying_key_bytes, 32)[12..]
```

Only the **32-byte seed** is stored. The full signing key (4032 bytes) is derived on load.

---

## Address Derivation

### Formula

```
address = shake256(verifying_key_bytes, 32)[12..]
```

SHAKE-256 (XOF) is used for cryptographic consistency with ML-DSA-65, which uses SHAKE-256 internally.

### Step by Step

```
1. Serialize the ML-DSA-65 verifying key → 1952 raw bytes
2. Compute hash = shake256(1952 bytes, 32) → 32 bytes
3. Take the last 20 bytes: hash[12..32]
4. Result: 20-byte Ethereum-compatible address
```

### Comparison with Classical Ethereum

| | Classical Ethereum | PostQuantumEVM |
|---|---|---|
| Key algorithm | secp256k1 (ECDSA) | ML-DSA-65 (Dilithium) |
| Public key size | 64 bytes | 1952 bytes |
| Hash function | keccak256 | SHAKE-256 |
| Input to hash | 64-byte ECDSA public key | 1952-byte ML-DSA-65 verifying key |
| Address size | 20 bytes | 20 bytes |
| Address format | Identical | Identical |

Addresses remain **20 bytes** and are indistinguishable from classical Ethereum addresses.

---

## Keystore Format

### Encryption Scheme

| Layer | Algorithm | Parameters |
|-------|-----------|------------|
| **KDF** | Argon2id | m=65536 (64MB), t=3 iterations, p=4 parallelism |
| **Cipher** | AES-256-GCM | Authenticated encryption |
| **Salt** | Random | 16 bytes |
| **Nonce/IV** | Random | 12 bytes |

### JSON Structure

```json
{
  "version": 1,
  "address": "0x0674683D469a021377196cf5a5D3a1F80Bcf7072",
  "public_key": "<hex 3904 chars = 1952 bytes>",
  "crypto": {
    "kdf": "argon2id",
    "kdf_params": { "m_cost": 65536, "t_cost": 3, "p_cost": 4, "salt": "<hex>" },
    "cipher": "aes-256-gcm",
    "cipher_params": { "iv": "<hex 12 bytes>" },
    "ciphertext": "<hex 64 bytes>"
  }
}
```

Only the **32-byte seed** is encrypted. Address and public key are stored in cleartext (public data).

---

## Transaction Signing

### Signing Flow

```
1. Construct PqTxRequest (unsigned transaction fields)
2. Compute signing_hash = shake256(0x50 || fields..., 32)
3. Sign: signature = ml_dsa_65_sign(signing_key, signing_hash) → 3309 bytes
4. Attach public key (1952 bytes) to the signed transaction
5. Compute tx_hash = shake256(0x50 || signing_hash || signature || public_key, 32)
6. Encode: 0x50 || RLP([chain_id, nonce, gas_price, gas_limit, to, value, input, sig, pk])
```

### Why the Public Key Is Embedded

ML-DSA signatures are **not recoverable** — unlike ECDSA, there is no way to derive the signer's public key from the signature alone. Therefore PQ transactions embed the full 1952-byte public key.

---

## CLI Commands (`pq-wallet`)

```bash
cd pq-wallet
cargo run --bin pq-wallet -- <command>
```

| Command | Description | Passphrase |
|---------|-------------|------------|
| `new` | Generate ML-DSA-65 keypair, save encrypted keystore | Yes |
| `address` | Show address from keystore | No |
| `balance` | Query qETH balance via RPC | No |
| `send` | Build, sign, broadcast a PQ transaction | Yes |
| `call` | Execute a read-only contract call (eth_call) | No |
| `sign` | Sign an arbitrary message | Yes |

### Examples

```bash
# Generate a new account
pq-wallet new --output keystore.json

# Check balance
pq-wallet balance --keystore keystore.json --rpc http://localhost:8545

# Send 1 qETH
pq-wallet send \
  --keystore keystore.json \
  --to 0x1111111111111111111111111111111111111111 \
  --value 1000000000000000000 \
  --rpc http://localhost:8545

# Deploy a contract
pq-wallet send \
  --keystore keystore.json \
  --data 0x<init-code-hex> \
  --gas-limit 200000 \
  --rpc http://localhost:8545

# Read-only contract call
pq-wallet call \
  --to 0x<contract-address> \
  --data 0x2e64cec1 \
  --rpc http://localhost:8545
```

---

## TUI Dashboard (`pq-tui`)

```bash
cd pq-wallet
cargo run --bin pq-tui
```

### Tabs

| Tab | Content |
|-----|---------|
| **Wallet** | Address, balance, public key, signature/key sizes |
| **Transactions** | Recent transactions (own), type, value, hash |
| **Blocks** | Block explorer (number, hash, gas used, tx count) |
| **Network** | Chain ID, block number, gas price, peer count |

### Interactive Actions

| Hotkey | Action | Requires Passphrase |
|--------|--------|---------------------|
| `s` | Send qETH transfer | Yes |
| `d` | Deploy contract | Yes |
| `c` | Call contract (read-only) | No |
| `r` | Refresh all data | No |
| `q` | Quit | No |

Actions open overlay forms. The passphrase is prompted once per session and cached in memory.

### Configuration

Environment variables:
- `PQ_RPC_URL` — RPC endpoint (default: `http://localhost:8545`)
- `PQ_KEYSTORE` — Keystore path (default: `../keystore.json`)

---

## Chain Seeder (`pq-seed`)

Pre-populates a running node with demo transactions for defense presentations.

```bash
cd pq-wallet
cargo run --bin pq-seed -- \
  --rpc http://localhost:8545 \
  --keystore keystore.json \
  --passphrase <pass> \
  --transfers 10 \
  --contract-calls 5 \
  --tx-delay 2
```

### What It Does

1. **Transfers** (Phase 1): Sends N transfers with varying amounts (0.01–1.0 qETH) to deterministic demo addresses
2. **Contract Deploy** (Phase 2): Deploys a SimpleStorage contract (hand-crafted EVM bytecode)
3. **Contract Calls** (Phase 3): Makes N `store(uint256)` calls, verifies final value with `eth_call`

### CLI Options

| Flag | Default | Description |
|------|---------|-------------|
| `--rpc` | `http://localhost:8545` | Node RPC endpoint |
| `--keystore` | `../keystore.json` | Keystore file path |
| `--passphrase` | (required) | Keystore passphrase (or `PQ_PASSPHRASE` env) |
| `--transfers` | `10` | Number of transfer txs |
| `--deploy-contract` | `true` | Whether to deploy SimpleStorage |
| `--contract-calls` | `5` | Number of store() calls |
| `--tx-delay` | `2` | Seconds between transactions |

---

## E2E Validation (`pq-e2e`)

```bash
cd pq-wallet
cargo run --bin pq-e2e -- --rpc http://localhost:8545
```

### Test Phases

| Phase | Tests | Description |
|-------|-------|-------------|
| 1. Chain Identity | 2 | Chain ID = 20561, genesis block exists |
| 2. PoA Consensus | 2 | Blocks advance, seal verification |
| 3. EIP-1559 Fees | 2 | Base fee exists, gas price > 0 |
| 4. PQ Transactions | 3 | Transfer, nonce increment, receipt |
| 5. Smart Contracts | 2 | Deploy + call |
| 6. Multi-Node | 1 | Block consistency across validators |

---

## Security Considerations

### Quantum Resistance

| Attack | Classical (ECDSA) | PostQuantumEVM (ML-DSA-65) |
|--------|-------------------|---------------------------|
| Shor's algorithm (ECDLP) | Key recovery | Not applicable |
| Grover's algorithm (hash) | 128-bit security | 128-bit security |
| Lattice reduction (LWE/SIS) | Not applicable | Primary threat — NIST Level 3 |

### Seed Security

- 256 bits of entropy (Grover reduces to 128-bit, still infeasible)
- Argon2id KDF (64MB memory, GPU/ASIC resistant)
- AES-256-GCM authenticated encryption
- Only the 32-byte seed is secret — everything else is derivable

---

## Library API (`pq-wallet-core`)

```rust
use pq_wallet_core::{PqKeypair, Keystore, PqSigner, PqTxRequest, RpcClient};

// Generate keypair
let keypair = PqKeypair::generate();
println!("Address: {:?}", keypair.address());

// Save/load keystore
keypair.save(Path::new("key.json"), "passphrase").unwrap();
let loaded = Keystore::load(Path::new("key.json"), "passphrase").unwrap();

// Sign a transaction
let tx = PqTxRequest {
    chain_id: 20561,
    nonce: 0,
    to: Some(recipient),
    value: 1_000_000_000_000_000_000u128,
    gas_limit: 21_000,
    gas_price: 1_000_000_000,
    input: vec![],
};
let signed = PqSigner::new(&keypair).sign(tx);
let raw_bytes = signed.encode(); // Ready for eth_sendRawTransaction

// RPC client
let rpc = RpcClient::new("http://localhost:8545");
let balance = rpc.get_balance(keypair.address()).await?;
let tx_hash = rpc.send_raw_transaction(&format!("0x{}", hex::encode(&raw_bytes))).await?;
```
