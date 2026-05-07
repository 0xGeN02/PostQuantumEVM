# pq-wallet — Post-Quantum Wallet for PostQuantumEVM

Command-line wallet, terminal UI, and tooling for the **PostQuantumEVM** network.
Generates ML-DSA-65 (CRYSTALS-Dilithium) keys, stores them encrypted on disk, and
constructs, signs, and broadcasts transactions of type `0x50`.

Keys **never leave the device** unencrypted. No server, no cloud service.

---

## Project Structure

```
pq-wallet/
├── pq-wallet-core/         # Reusable library
│   └── src/
│       ├── keygen.rs       # ML-DSA-65 keypair generation
│       ├── keystore.rs     # Encrypted keystore (AES-256-GCM + Argon2id)
│       ├── signer.rs       # Transaction signing
│       ├── tx.rs           # PqTxRequest / PqSignedTx types
│       ├── rpc.rs          # Minimal JSON-RPC client
│       └── error.rs        # WalletError
├── pq-wallet-cli/          # CLI binary (pq-wallet)
├── pq-wallet-tui/          # Terminal UI dashboard (pq-tui)
├── pq-chain-seeder/        # Chain seeder for demos (pq-seed)
└── pq-e2e-test/            # E2E validation framework (pq-e2e)
```

---

## Installation

```bash
cd pq-wallet
cargo build --release

# Binaries:
#   target/release/pq-wallet   (CLI)
#   target/release/pq-tui      (TUI dashboard)
#   target/release/pq-seed     (Chain seeder)
#   target/release/pq-e2e      (E2E tests)
```

---

## CLI Commands (`pq-wallet`)

### `new` — Generate keypair and save keystore

```bash
pq-wallet new --output keystore.json
# Prompts for passphrase interactively

pq-wallet new --output keystore.json --passphrase "my-secure-pass"
```

### `address` — Show address (no passphrase needed)

```bash
pq-wallet address --keystore keystore.json
# 0x0674683D469a021377196cf5a5D3a1F80Bcf7072
```

### `balance` — Query qETH balance

```bash
pq-wallet balance --keystore keystore.json --rpc http://localhost:8545
# Address: 0x0674683D469a...
# Balance: 10000.0000 qETH
```

### `send` — Build, sign, and broadcast transaction

```bash
# Simple transfer
pq-wallet send \
    --keystore keystore.json \
    --to 0x1111111111111111111111111111111111111111 \
    --value 1000000000000000000 \
    --rpc http://localhost:8545

# Contract deployment (no --to, pass init code via --data)
pq-wallet send \
    --keystore keystore.json \
    --data 0x608060405234... \
    --gas-limit 200000 \
    --rpc http://localhost:8545

# Dry run (sign but don't broadcast)
pq-wallet send --keystore keystore.json --to 0x... --value 0 --dry-run
```

### `call` — Read-only contract call

```bash
pq-wallet call \
    --to 0x<contract-address> \
    --data 0x2e64cec1 \
    --rpc http://localhost:8545
```

### `sign` — Sign arbitrary message

```bash
pq-wallet sign --keystore keystore.json "Hello post-quantum world"
```

---

## TUI Dashboard (`pq-tui`)

```bash
cargo run --bin pq-tui
```

Interactive terminal UI with 4 tabs:

| Tab | Content |
|-----|---------|
| Wallet | Address, balance, PK size, sig size |
| Transactions | Own transactions (hash, value, type) |
| Blocks | Block explorer (number, hash, gas, txs) |
| Network | Chain ID, block height, gas price |

### Hotkeys

| Key | Action |
|-----|--------|
| `s` | Send qETH (prompts for passphrase, recipient, amount) |
| `d` | Deploy contract (prompts for passphrase, init code) |
| `c` | Call contract (read-only, no passphrase) |
| `r` | Refresh all data |
| `q` | Quit |
| `Tab` / arrows | Navigate tabs |

---

## Chain Seeder (`pq-seed`)

Pre-populates the chain with demo transactions for presentations:

```bash
cargo run --bin pq-seed -- \
  --rpc http://localhost:8545 \
  --keystore keystore.json \
  --passphrase <pass> \
  --transfers 10 \
  --contract-calls 5
```

**What it does:**
1. Sends N transfers (0.01–1.0 qETH) to demo addresses
2. Deploys a SimpleStorage contract (hand-crafted EVM bytecode)
3. Makes N `store(uint256)` calls to the contract
4. Verifies the stored value via `eth_call`

---

## E2E Validation (`pq-e2e`)

```bash
cargo run --bin pq-e2e -- --rpc http://localhost:8545
```

Runs 12 test scenarios across 6 phases: chain identity, PoA consensus, EIP-1559 fees,
PQ transactions, smart contracts, and multi-node consistency.

---

## Keystore Format

```json
{
  "version": 1,
  "address": "0x0674683D469a...",
  "public_key": "<hex 1952 bytes>",
  "crypto": {
    "kdf": "argon2id",
    "kdf_params": { "m_cost": 65536, "t_cost": 3, "p_cost": 4, "salt": "<hex>" },
    "cipher": "aes-256-gcm",
    "cipher_params": { "iv": "<hex 12 bytes>" },
    "ciphertext": "<hex 64 bytes>"
  }
}
```

Only the **32-byte seed** is encrypted. The full signing key (4032 bytes) is
deterministically derived from the seed on load via `MlDsa65::from_seed(&seed)`.

---

## Running Tests

```bash
cd pq-wallet
cargo test --workspace
```

---

## Security Notes

- Experimental code — not for production use with real funds.
- Passphrase is read from stdin without echo. Avoid `--passphrase` in production
  (visible in shell history).
- The keystore file alone is not secret without the passphrase. Store securely anyway.
- Backup keystore AND passphrase separately.
