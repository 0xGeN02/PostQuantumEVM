# TODO - PostQuantumEVM Integration

## Overall Status: ~98% complete — All 6 Phases DONE + Gas Analysis + PoA Consensus

All phases are complete. The PQ node starts in dev mode, mines blocks with PQ transactions, has the PQHASH opcode (0x21), all classical precompiles are disabled, E2E transaction flow works, Solidity contracts are ready, and multi-node Docker is configured with PoA consensus.

### What works end-to-end:
- ML-DSA-65 key generation → SHAKE-256 address derivation
- Transaction signing with ML-DSA-65 + SHAKE-256 hashing  
- RLP encoding with PQ_TX_TYPE=0x50 → broadcast → pool → block inclusion → state change
- Wallet CLI: new, address, balance, send, deploy, receipt, sign
- Solidity contracts: PQVerify, PQHash, PQMultiSig, PQAccessControl
- PoA consensus: round-robin validators with ML-DSA-65 block sealing
- Docker multi-validator setup (3 PoA validators, round-robin rotation)

---

## Critical (Blocking node execution)

- [x] **PQ node executable binary** — DONE
  - `pq-reth/bin/pq-reth/src/main.rs` launches `PqNode` with the `NodeBuilder`
  - Compiles and runs successfully in `--dev` mode

- [x] **Genesis / PQ Chain Spec** — DONE
  - `pq-reth/bin/pq-reth/genesis.json` with chain_id=20561 (0x5051)
  - 8 pre-funded accounts (10,000 ETH each)
  - All hardforks activated from genesis (including Prague)

---

## PQ Wallet

- [x] **Fix wire format (CRITICAL)** — DONE
  - Wallet now produces: `0x50 || RLP([chain_id, nonce, gas_price, gas_limit, to, value, input, sig, pk])`
  - Compatible with node's `Decodable2718` implementation

- [ ] **Secure passphrase input**
  - Passphrase input does not hide characters (missing `rpassword` crate)

- [x] **Contract creation support** — DONE
  - The `deploy` command sends tx with `to: None` and `--code` for init bytecode

---

## EVM / Opcodes / Solidity

- [x] **Solidity interface contracts for the precompile** — DONE
  - `contracts/src/PQVerify.sol` — ML-DSA-65 verify library calling `0x0100`
  - `contracts/src/PQHash.sol` — SHAKE-256 helper via PQHASH opcode
  - `contracts/src/PQMultiSig.sol` — M-of-N multi-sig with ML-DSA-65
  - `contracts/src/PQAccessControl.sol` — Role-based access control with PQ sigs

- [x] **Impact of disabled ecrecover** — DOCUMENTED
  - `ecrecover()` always reverts — breaks existing contracts using ECDSA
  - OpenZeppelin `ECDSA.recover()`, ERC-2612 permit, EIP-712 — all broken
  - PQ alternative: `PQVerify.verify(msgHash, sig, pk)` via precompile at `0x0100`
  - Documented in `contracts/README.md` and `docs/GAS_COST_ANALYSIS.md`

- [x] **Precompile output — Solidity compatibility** — DONE
  - Upgraded from 1 byte to 32 bytes (left-padded uint256)
  - Compatible with `abi.decode(result, (uint256))` pattern
  - Returns `1` for valid, `0` for invalid signature

- [x] **Disable classical elliptic curve precompiles (13 precompiles)** — DONE
  - ALL 13 classical precompiles disabled in `pq_precompiles()` in `reth-pq-evm/src/lib.rs`
  - **BN254 (broken):**
    - `0x06` ecAdd — point addition on classical curve
    - `0x07` ecMul — scalar multiplication on classical curve
    - `0x08` ecPairing — pairing check (used in Groth16 SNARKs)
  - **KZG (broken):**
    - `0x0a` point_evaluation (EIP-4844) — relies on DLP over BLS12-381
  - **BLS12-381 (all broken):**
    - `0x0b` bls12_g1Add
    - `0x0c` bls12_g1Mul
    - `0x0d` bls12_g1Msm
    - `0x0e` bls12_g2Add
    - `0x0f` bls12_g2Mul
    - `0x10` bls12_g2Msm
    - `0x11` bls12_pairing
    - `0x12` bls12_map_fp_to_g1
    - `0x13` bls12_map_fp2_to_g2
  - All must return error like ecrecover (stub with "disabled on PQ chain" message)
  - Any contract relying on them for security has zero protection against a quantum adversary
  - Implement in `pq_precompiles()` in `reth-pq-evm/src/lib.rs`

- [x] **Precompiles KEPT (quantum-safe)** — DONE
  - `0x02` SHA-256 — Grover reduces to 128-bit (sufficient)
  - `0x03` RIPEMD-160 — hash
  - `0x04` Identity — data copy only
  - `0x05` ModExp — pure arithmetic
  - `0x09` Blake2f — hash compression function

- [x] **New opcode `0x21 PQHASH` — native SHAKE-256 in the EVM** — DONE
  - Opcode `0x21` computing SHAKE-256 (same hash ML-DSA uses internally)
  - Stack: `(offset, length) → hash_256` (same interface as KECCAK256 at 0x20)
  - Gas: 30 base + 6 per word (same model as KECCAK256)
  - Implemented via `evm.instruction.insert_instruction(0x21, ...)` in `PqEvmFactory`

- [x] **Migrate PQ protocol hashing to SHAKE-256** — DONE
  - Address derivation: `shake256(pk, 32)[12..]` (both node and wallet)
  - Transaction signing hash: SHAKE-256
  - Transaction hash: SHAKE-256
  - State trie kept using keccak256 (too invasive to change)

- [x] **Existing EVM opcodes — analysis completed**
  - No EVM opcode performs elliptic curve operations internally
  - CALLER/ORIGIN work correctly (read address from TxEnv after ML-DSA recovery)
  - CREATE/CREATE2 use only hashing to derive addresses
  - KECCAK256 (0x20) is kept for legacy contract compatibility
  - New PQHASH (0x21) provides the native post-quantum alternative

---

## Precompiled Contracts

- [x] **Gas benchmark and calibration for ML-DSA precompile** — DONE
  - Calibrated from 50,000 → 3,450 gas based on criterion benchmarks
  - ML-DSA-65 verify = ~42µs (faster than ecrecover's ~49µs)
  - See `docs/GAS_COST_ANALYSIS.md` and `benchmarks/benches/crypto_ops.rs`

- [ ] **ML-DSA batch verify precompile (`0x0101`)**
  - Verify N signatures in a single call with amortized cost (~30-40% savings)
  - Critical for multi-sig wallets, batch settlements, rollup verification
  - Input: `[N:u32 | (msg_hash:32 | sig:3309 | pk:1952) × N]`

- [ ] **ML-KEM-768 decapsulate precompile (`0x0102`)**
  - On-chain key exchange for encrypted messaging, sealed-bid auctions, privacy
  - ML-KEM is already implemented in `ml-lattice-rs` — only the precompile is missing
  - Input: `[ciphertext:1088 | decapsulation_key:2400]` → Output: `shared_secret:32`

- [ ] **Formal precompile documentation (EIP-style)**
  - Define exact encoding (byte layout, endianness)
  - Specify ML-DSA mode (pure vs pre-hash)
  - Domain separation / context strings
  - Gas pricing formula

---

## Kubernetes / Multi-Node Deployment

- [x] **PQ node Dockerfile** — DONE
  - `Dockerfile.pq-reth` — multi-stage build (compile + minimal runtime image)
  - Builds the PQ binary directly

- [x] **Genesis JSON for the PQ testnet** — DONE
  - `pq-reth/bin/pq-reth/genesis.json` — chain_id=20561, Prague from genesis
  - 10 pre-funded accounts (10,000 ETH each), gasLimit=36M

- [ ] **Kubernetes manifests** (deferred — Docker Compose is sufficient for demo)
  - StatefulSet, Headless Service, PVCs, ConfigMap, Init container
  - Not blocking; would be needed for production deployment

- [x] **Consensus strategy for the demo** — DONE
  - PoA consensus with round-robin ML-DSA-65 block sealing
  - `reth-pq-poa` crate: ValidatorSet, Sealer, PoaEngine, PoaMiningStream
  - Integrates via `MiningMode::Trigger` — only mines on validator's turn
  - Falls back to `--dev` auto-mine when no PoA config provided

- [x] **Multi-node Docker Compose** — DONE
  - `docker-compose.yml` with 3 PoA validators (round-robin rotation)
  - Each validator has its own ML-DSA-65 key and PQ_POA_CONFIG
  - Bridge network, persistent volumes, healthchecks
  - `scripts/generate-validator-keys.sh` for key generation

- [x] **Ports exposed per node** — DONE
  - 30303 TCP+UDP (P2P), 8545 TCP (HTTP RPC), 8546 TCP (WebSocket)

---

## CLI for blockchain interaction

- [x] **Core commands in pq-wallet-cli** — DONE
  - `new` — generate ML-DSA-65 keypair + encrypted keystore
  - `address` — show address from keystore
  - `balance` — query ETH balance
  - `send` — sign and broadcast PQ transaction
  - `deploy` — contract creation (to: None + --code)
  - `receipt` — query tx receipt by hash
  - `sign` — sign arbitrary message

- [x] **Additional commands** — PARTIAL
  - `call` — `eth_call` read-only for contract view functions ✅
  - `block` — show current block or by number
  - `nonce` — query nonce directly
  - `status` — combined node info (chain_id, block number, gas price, peers)
  - `accounts` / `list` — list available keystores

- [x] **Core RPC methods in pq-wallet-core** — DONE
  - `eth_getBalance`
  - `eth_getTransactionCount` (nonce)
  - `eth_sendRawTransaction`
  - `eth_getTransactionReceipt`
  - `eth_gasPrice`

- [x] **Additional RPC methods** — PARTIAL
  - `eth_call` (contract reads) ✅
  - `eth_getBlockByNumber` / `eth_blockNumber`
  - `eth_getCode` (check if address is a contract)
  - `eth_estimateGas`
  - `net_peerCount` / `net_version`
  - `eth_getLogs` (event querying)

- [x] **Automated demo script** — DONE
  - `scripts/demo.sh` — full flow: keygen → balance → send → deploy → receipt

---

## Important (Required for production)

- [x] **Pool Validator with real state** — DONE
  - `PqPoolValidator<Client>` now queries `StateProviderFactory` for nonce/balance
  - Rejects replay (nonce too low) and overdraft (insufficient balance)
  - Generic over any `StateProviderFactory` client

- [ ] **Integration / E2E tests**
  - Only ~17 unit tests exist in the PQ crates
  - Missing: test node starting, processing blocks with PQ txs, node synchronization
  - Missing: end-to-end test wallet → RPC → pool → block → receipt

---

## Medium (Quality and correctness)

- [ ] **Full RLP serialization in signing_hash**
  - The `signing_hash` uses simplified encoding, not full canonical RLP
  - Production should use `alloy-rlp` for compatibility with existing tooling

- [ ] **P2P - Message size limit adjustments**
  - PQ txs are ~5.3KB (vs ~100-200B classical)
  - May need adjustments to message limits and propagation
  - ML-KEM available but not used for P2P encryption

- [ ] **RPC - Native PQ transaction conversion**
  - `PqRpcTxConverter` converts PQ txs to `TxEnvelope::Legacy` with dummy ECDSA signatures
  - Implement native RPC format for type `0x04` transactions

---

## Completed

- [x] PQ Library (`ml-lattice-rs`) — ML-DSA-65 + ML-KEM-768
- [x] PQ Wallet (`pq-wallet`) — CLI with encrypted keystore (Argon2id + AES-256-GCM)
- [x] Qiskit Simulation — Shor + Grover attack demos
- [x] `reth-pq-primitives` — Tx type `0x50`, ML-DSA-65 signatures, codecs
- [x] `reth-pq-consensus` — PQ signature validation
- [x] `reth-pq-precompile` — ML-DSA verify precompile at `0x0100` (32-byte ABI output)
- [x] `reth-pq-evm` — ecrecover disabled, PQ precompile injected, PQHASH opcode
- [x] `reth-pq-node` — `PqNode` with all components wired + DebugNode for dev mode
- [x] `reth-pq-node-primitives` — `PqPrimitives` impl
- [x] `reth-pq-pool` — Pool base structure (partial validation)
- [x] Successful compilation of all PQ crates
- [x] EVM opcode analysis completed
- [x] Addresses remain 20 bytes (compatible with existing tooling)
- [x] **PQ node binary (`bin/pq-reth/`) — launches PqNode, starts in --dev mode**
- [x] **Genesis file (chain_id=20561, pre-funded accounts, Prague from genesis)**
- [x] **PQHASH opcode (0x21) — native SHAKE-256 in the EVM**
- [x] **Migrated all PQ hashing to SHAKE-256 (address, signing_hash, tx_hash)**
- [x] **Disabled 13 classical elliptic curve precompiles (Shor-vulnerable)**
- [x] **Wallet wire format fixed — proper RLP encoding (EIP-2718)**
- [x] **PQ_TX_TYPE changed from 0x04 to 0x50 (avoids EIP-7702 collision)**
- [x] **E2E transaction: wallet → pool → block → balance change (VERIFIED)**
- [x] **Wallet CLI: deploy + receipt commands**
- [x] **Solidity contracts: PQVerify, PQHash, PQMultiSig, PQAccessControl**
- [x] **ML-DSA precompile output upgraded to 32 bytes (ABI-compatible uint256)**
- [x] **Multi-node Docker Compose (3 PoA validators, round-robin rotation)**
- [x] **Automated demo script (scripts/demo.sh)**
- [x] **PoA consensus engine (`reth-pq-poa`) — ML-DSA-65 block sealing**
- [x] **PoA integration: PoaMiningStream + MiningMode::Trigger in main.rs**
- [x] **Gas cost analysis paper (docs/GAS_COST_ANALYSIS.md)**
- [x] **Benchmark visualization notebook (docs/benchmark_analysis.ipynb)**
- [x] **Consensus documentation (docs/CONSENSUS.md) — PoS analysis + PoA justification**
- [x] **Pool validator with real state (nonce + balance checks via StateProviderFactory)**
- [x] **PoA seal verification (PqPoaConsensus wraps inner consensus, verifies ML-DSA-65 seals)**
- [x] **Wallet eth_call + CLI call command (read-only contract calls)**

---

## Recommended execution order

### Phase 1 — Functional node ✅ COMPLETE
1. ~~Create the `pq-reth` binary that launches `PqNode`~~ ✅
2. ~~Create genesis.json with PQ chain spec (pre-fund demo accounts)~~ ✅
3. ~~Verify the node starts in `--dev` mode and produces blocks~~ ✅

### Phase 2 — SHAKE-256 and new opcode ✅ COMPLETE
4. ~~Implement opcode `0x21 PQHASH` (SHAKE-256) in `reth-pq-evm`~~ ✅
5. ~~Migrate PQ address derivation to SHAKE-256 (`reth-pq-primitives`, `pq-wallet`)~~ ✅
6. ~~Migrate tx hashing (signing_hash, tx_hash) to SHAKE-256~~ ✅
7. ~~Disable classical curve precompiles (0x06-0x08, 0x0a-0x13)~~ ✅

### Phase 3 — Compatible wallet ✅ COMPLETE
8. ~~Fix wallet wire format (RLP encoding compatible with the node)~~ ✅
9. ~~Send a PQ tx from wallet to node and verify execution~~ ✅
10. ~~Add `deploy` and `receipt` commands to the CLI~~ ✅

### Phase 4 — PQ Smart Contracts ✅ COMPLETE
11. ~~Create Solidity library `PQVerify.sol` for the precompile~~ ✅
12. ~~Deploy sample contract and verify PQ signature on-chain~~ ✅
13. ~~Evaluate precompile output (1 byte vs 32 bytes)~~ ✅ — upgraded to 32 bytes

### Phase 5 — Multi-node ✅ COMPLETE
14. ~~PQ node Dockerfile~~ ✅
15. ~~Docker Compose with 3-4 nodes (1 producer + followers)~~ ✅
16. ~~Shared genesis and peer discovery with bootnodes~~ ✅
17. Kubernetes manifests (deferred — Docker Compose sufficient for demo)

### Phase 6 — Full demo ✅ COMPLETE
18. ~~Automated demo script (create wallets → send txs → deploy contract → verify)~~ ✅
19. ~~CLI with all required commands~~ ✅
20. Gas benchmarks and calibration (deferred — needs production hardware)
21. Formal documentation and spec (covered in code comments + TODO.md)
