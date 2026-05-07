# PostQuantumEVM

## A Post-Quantum Cryptography EVM Blockchain

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/Python_3.13-3776AB?logo=python&logoColor=white&style=for-the-badge)](https://python.org)
[![Solidity](https://img.shields.io/badge/Solidity-503D8B?style=for-the-badge&logo=ethereum&logoColor=white)](https://docs.soliditylang.org/en/latest/)
[![License: MIT](https://img.shields.io/badge/License-APACHE_2.0-yellow.svg?style=for-the-badge)](LICENSE)

</div>

> [!IMPORTANT]
> **Preparing blockchain systems for the post-quantum era.**
> This project replaces all ECDSA/secp256k1 primitives with ML-DSA-65 (NIST FIPS 204)
> and demonstrates a fully post-quantum resistant EVM chain.

---

## Overview

**PostQuantumEVM** is an Ethereum execution client hardened against quantum adversaries.
Built as a non-invasive extension of [reth](https://github.com/paradigmxyz/reth), it
replaces every classical elliptic-curve primitive with NIST-standardized post-quantum
algorithms while maintaining full EVM compatibility.

| Classical (Ethereum) | Post-Quantum (PQ-EVM) | Standard |
|---|---|---|
| ECDSA / secp256k1 | ML-DSA-65 (Dilithium) | NIST FIPS 204 |
| ECDH / secp256k1 | ML-KEM-768 (Kyber) | NIST FIPS 203 |
| Keccak-256 (protocol hash) | SHAKE-256 | NIST FIPS 202 |
| ecrecover precompile | ML-DSA verify precompile (`0x0100`) | --- |

**Chain ID:** `20561` | **Native Token:** qETH (18 decimals) | **Tx Type:** `0x50`

---

## Quick Start

### Prerequisites

- Rust 1.84+ (rustup)
- Docker 24+ (for multi-validator deployment)
- Git (with submodule support)

### 1. Clone

```bash
git clone --recurse-submodules https://github.com/0xGeN02/PostQuantumEVM.git
cd PostQuantumEVM
```

### 2. Run a Single Node (Dev Mode)

```bash
cd pq-reth
cargo run -p pq-reth --bin pq-reth -- node \
  --chain bin/pq-reth/genesis.json \
  --dev \
  --dev.block-time 5s \
  --http \
  --http.api eth,net,web3
```

The node starts on `http://localhost:8545` with a pre-funded genesis (10k qETH per account).

### 3. Use the Wallet

```bash
cd pq-wallet

# Generate a new ML-DSA-65 keypair
cargo run --bin pq-wallet -- new --output keystore.json

# Check balance
cargo run --bin pq-wallet -- balance --keystore keystore.json --rpc http://localhost:8545

# Send a transaction
cargo run --bin pq-wallet -- send \
  --keystore keystore.json \
  --to 0x1111111111111111111111111111111111111111 \
  --value 1000000000000000000 \
  --rpc http://localhost:8545
```

### 4. Launch the TUI Dashboard

```bash
cd pq-wallet
cargo run --bin pq-tui
```

4-tab terminal dashboard: Wallet, Transactions, Blocks, Network.
Hotkeys: `s`=Send, `d`=Deploy, `c`=Call, `r`=Refresh, `q`=Quit.

### 5. Multi-Validator PoA (Docker)

```bash
# Generate 3 validator keys
./scripts/generate-validator-keys.sh

# Build and start 3 validators
docker compose build
docker compose up -d

# RPC endpoints:
#   Validator 1: http://localhost:8545
#   Validator 2: http://localhost:8546
#   Validator 3: http://localhost:8547
```

### 6. Seed the Chain with Demo Data

```bash
cd pq-wallet
cargo run --bin pq-seed -- \
  --rpc http://localhost:8545 \
  --keystore keystore.json \
  --passphrase <your-passphrase> \
  --transfers 10 \
  --contract-calls 5
```

Sends transfers (varying amounts), deploys a SimpleStorage contract, and makes store() calls.

---

## Architecture

```
PostQuantumEVM/
├── pq-reth/                        # Forked reth (git submodule)
│   ├── bin/pq-reth/                # Binary + genesis.json
│   └── crates/pq/
│       ├── reth-pq-primitives      # PqSignedTransaction, RLP, Compact codec
│       ├── reth-pq-consensus       # ML-DSA-65 transaction validation
│       ├── reth-pq-precompile      # ML-DSA-65 verify precompile at 0x0100
│       ├── reth-pq-pool            # Mempool validator (sig verification + state)
│       ├── reth-pq-evm             # PqEvmFactory, disabled classical precompiles
│       ├── reth-pq-node-primitives # PqPrimitives (NodePrimitives impl)
│       ├── reth-pq-node            # PqNode, engine, RPC, payload builder
│       └── reth-pq-poa             # PoA engine (ML-DSA-65 block sealing)
├── ml-lattice-rs/                  # PQ crypto library (git submodule)
│   ├── dilithium/                  # ML-DSA-65 (FIPS 204)
│   └── kyber/                      # ML-KEM-768 (FIPS 203)
├── pq-wallet/                      # Wallet ecosystem
│   ├── pq-wallet-core/             # Core library (keygen, signer, RPC, tx)
│   ├── pq-wallet-cli/              # CLI binary (pq-wallet)
│   ├── pq-wallet-tui/              # TUI dashboard (pq-tui)
│   ├── pq-chain-seeder/            # Chain seeder for demos (pq-seed)
│   └── pq-e2e-test/                # E2E validation framework (pq-e2e)
├── qiskit-api/                     # Quantum attack simulation (Shor/Grover)
├── contracts/                      # Solidity PQ precompile interfaces (Foundry)
├── benchmarks/                     # Criterion.rs benchmarks
├── e2e/                            # E2E orchestration
│   ├── k8s/                        # Kubernetes manifests (3-validator cluster)
│   └── run-e2e.sh                  # Orchestration script
├── scripts/                        # Tooling scripts
│   └── generate-validator-keys.sh  # ML-DSA-65 validator key generation
├── Dockerfile.pq-reth              # Multi-stage Docker build
└── docker-compose.yml              # 3-validator PoA Docker Compose
```

---

## Transaction Format

PQ transactions use EIP-2718 type **`0x50`** (`'P'` for Post-Quantum).

```
0x50 || RLP([chain_id, nonce, gas_price, gas_limit, to, value, input,
             signature (3309 B), public_key (1952 B)])
```

| Field | Classical tx | PQ tx |
|---|---|---|
| Signature | 65 B (r, s, v) | 3 309 B |
| Public key | 0 B (recovered via ecrecover) | 1 952 B |
| Total overhead | ~110 B | ~5 314 B |

---

## Consensus: Proof of Authority (PoA)

PostQuantumEVM uses **round-robin PoA** with ML-DSA-65 block sealing:

- Fixed validator set (3 by default)
- Each validator signs blocks on their turn
- 5-second slot time (configurable via `slot_time_ms`)
- Block seal = ML-DSA-65 signature over the block hash
- No BLS aggregation (quantum-vulnerable)

Configuration: `PQ_POA_CONFIG=/path/to/poa-config.json`

---

## Key Sizes

| | Classical Ethereum | PostQuantumEVM | Ratio |
|---|---|---|---|
| Private key | 32 B | 4 032 B | 126x |
| Public key | 64 B | 1 952 B | 30x |
| Signature | 65 B | 3 309 B | 51x |
| Address | 20 B | 20 B | **1x** |

Address derivation: `shake256(public_key_bytes, 32)[12..]`

---

## Documentation

| Document | Description |
|---|---|
| [docs/README.md](docs/README.md) | Full technical documentation index |
| [docs/NODE.md](docs/NODE.md) | Node architecture, Docker/K8s deployment |
| [docs/WALLET.md](docs/WALLET.md) | Wallet, TUI, keystore, signing |
| [docs/CONSENSUS.md](docs/CONSENSUS.md) | PoA consensus mechanism |
| [docs/GAS_COST_ANALYSIS.md](docs/GAS_COST_ANALYSIS.md) | Gas pricing and throughput analysis |
| [e2e/k8s/README.md](e2e/k8s/README.md) | Kubernetes deployment guide |

---

## Author

**0xGeN02**
Building secure and scalable systems at the intersection of cryptography and distributed systems.
[GitHub Profile](https://github.com/0xGeN02)

---

<div align="center">

*Preparing blockchain systems for the quantum era.*

</div>
