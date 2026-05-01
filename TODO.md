# TODO - PostQuantumEVM Integration

## Overall Status: ~85-90% complete

Phases 1 and 2 are complete. The PQ node starts, produces blocks, has the PQHASH opcode, and all classical precompiles are disabled. Next: wallet wire format fix and multi-node deployment.

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

- [ ] **Fix wire format (CRITICAL)**
  - Wallet produces: `0x04 || signing_hash(32) || sig(3309) || pk(1952)`
  - Node expects: `0x04 || RLP([chain_id, nonce, gas_price, gas_limit, to, value, input, sig, pk])`
  - Rewrite `PqSignedTx::encode()` to produce RLP compatible with `reth-pq-primitives/src/rlp.rs`
  - Add `alloy-rlp` as a dependency

- [ ] **Secure passphrase input**
  - Passphrase input does not hide characters (missing `rpassword` crate)

- [ ] **Contract creation support**
  - The `send` command always requires `--to`
  - `PqTxRequest` already has `to: Option<Address>`, but the CLI does not allow omitting it

---

## EVM / Opcodes / Solidity

- [ ] **Solidity interface contracts for the precompile**
  - No `.sol` files exist in the project
  - Create a Solidity library with an interface to call the ML-DSA precompile at `0x0100`
  - Example of on-chain PQ verification for developers
  - Create sample contracts (e.g., PQ multi-sig, PQ access control)

- [ ] **Impact of disabled ecrecover**
  - `ecrecover()` always reverts — breaks existing contracts using ECDSA
  - OpenZeppelin `ECDSA.recover()`, ERC-2612 permit, EIP-712 — all broken
  - Document which Solidity patterns no longer work and their PQ alternatives
  - Consider creating a `PQSignatureChecker.sol` library (OpenZeppelin equivalent)

- [ ] **Precompile output — Solidity compatibility**
  - Currently returns 1 byte (`0x00`/`0x01`)
  - Convention is to return 32 bytes (left-padded) for easier `abi.decode`
  - Evaluate switching to 32-byte output for better Solidity ergonomics

- [ ] **Disable classical elliptic curve precompiles (12 precompiles)**
  - ~~Currently only `0x01` (ecrecover) is disabled, but 12 more use classical crypto broken by Shor's algorithm~~
  - [x] **ALL 13 classical precompiles now disabled** — implemented in `pq_precompiles()` in `reth-pq-evm/src/lib.rs`
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

- [ ] **Precompiles to KEEP (quantum-safe)**
  - `0x02` SHA-256 — hash, Grover reduces to 128-bit (sufficient)
  - `0x03` RIPEMD-160 — hash
  - `0x04` Identity — data copy only
  - `0x05` ModExp — pure arithmetic (not a security primitive by itself)
  - `0x09` Blake2f — hash compression function
  - Document why they are kept (quantum security justification)

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

- [ ] **Gas benchmark and calibration for ML-DSA precompile**
  - 50,000 gas is a placeholder
  - ecrecover = 3,000 gas (~0.03ms), ML-DSA-65 verify = ~0.3-0.5ms
  - Needs real benchmarks with criterion to derive actual cost
  - Estimate: ~15,000-50,000 gas depending on target hardware

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

- [ ] **PQ node Dockerfile**
  - `pq-reth/Dockerfile` exists but builds the standard `reth` binary
  - Modify to build the PQ binary (once it exists)
  - Multi-stage build: compile + minimal runtime image

- [ ] **Genesis JSON for the PQ testnet**
  - Unique chain ID (e.g., `0x5051` = "PQ")
  - Initial allocs for demo accounts (pre-funded)
  - Hardfork configuration (Prague activated from block 0)
  - Gas limit, block time

- [ ] **Kubernetes manifests**
  - Do not exist — create from scratch in `k8s/` directory
  - **StatefulSet** for nodes (3-4 replicas)
  - **Headless Service** for stable DNS between pods
  - **PersistentVolumeClaims** for chain data
  - **ConfigMap** with genesis.json and shared configuration
  - **Service** (LoadBalancer/NodePort) for external RPC access
  - **Init container** to extract enode URL from bootnode

- [ ] **Consensus strategy for the demo**
  - Use reth's `--dev` mode (auto-mine, no external CL required)
  - 1 producer node (`--dev --dev.block-time 5s`)
  - 2-3 follower nodes syncing via P2P
  - Peer discovery via `--bootnodes enode://<key>@<host>:30303`
  - NO Lighthouse/Prysm needed for the demo

- [ ] **Multi-node Docker Compose** (simpler alternative)
  - Before K8s, have a `docker-compose.yml` with 3-4 nodes for local development
  - Bridge network between containers
  - Persistent volumes

- [ ] **Ports to expose per node**
  - 30303 TCP+UDP — P2P (devp2p)
  - 8545 TCP — HTTP JSON-RPC
  - 8546 TCP — WebSocket JSON-RPC
  - 9001 TCP — Metrics (Prometheus)
  - 8551 TCP — Engine API (only if using CL)

---

## CLI for blockchain interaction

- [ ] **New commands needed in pq-wallet-cli**
  - `deploy` — send tx with `to: None` to deploy contracts, show contract address
  - `call` — `eth_call` read-only for contract view functions
  - `receipt` — query tx receipt by hash (with polling)
  - `block` — show current block or by number
  - `nonce` — query nonce directly
  - `status` — combined node info (chain_id, block number, gas price, peers)
  - `accounts` / `list` — list available keystores

- [ ] **Missing RPC methods in pq-wallet-core**
  - `eth_call` (contract reads)
  - `eth_getTransactionReceipt`
  - `eth_getBlockByNumber` / `eth_blockNumber`
  - `eth_getCode` (check if address is a contract)
  - `eth_estimateGas`
  - `net_peerCount` / `net_version`
  - `eth_getLogs` (event querying)

- [ ] **Automated demo script**
  - Bash/Python script demonstrating the full flow:
    1. Create 2 PQ keystores
    2. Show balances (pre-funded from genesis)
    3. Send ETH between PQ accounts
    4. Deploy a simple contract
    5. Call the ML-DSA precompile from a contract
    6. Query receipts and chain state

---

## Important (Required for production)

- [ ] **Pool Validator with real state**
  - `PqPoolValidator` uses `balance = U256::MAX` as a placeholder
  - Needs to query actual state (nonce, balance) to prevent double-spending
  - Without this, no protection against replay/spam

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
- [x] `reth-pq-primitives` — Tx type `0x04`, ML-DSA-65 signatures, codecs
- [x] `reth-pq-consensus` — PQ signature validation
- [x] `reth-pq-precompile` — ML-DSA verify precompile at `0x0100`
- [x] `reth-pq-evm` — ecrecover disabled, PQ precompile injected
- [x] `reth-pq-node` — `PqNode` with all components wired
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

### Phase 3 — Compatible wallet
8. Fix wallet wire format (RLP encoding compatible with the node)
9. Send a PQ tx from wallet to node and verify execution
10. Add `deploy` and `receipt` commands to the CLI

### Phase 4 — PQ Smart Contracts
11. Create Solidity library `PQVerify.sol` for the precompile
12. Deploy sample contract and verify PQ signature on-chain
13. Evaluate precompile output (1 byte vs 32 bytes)

### Phase 5 — Multi-node
14. PQ node Dockerfile
15. Docker Compose with 3-4 nodes (1 producer + followers)
16. Shared genesis and peer discovery with bootnodes
17. Migrate to Kubernetes manifests

### Phase 6 — Full demo
18. Automated demo script (create wallets → send txs → deploy contract → verify)
19. CLI with all required commands
20. Gas benchmarks and calibration for precompile + PQHASH opcode
21. Formal documentation and spec (precompile + opcode)
