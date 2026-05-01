# Consensus Mechanism: From Ethereum PoS to PQ-EVM PoA

## 1. Ethereum's Proof of Stake — How It Works

### 1.1 Dual-Client Architecture (Post-Merge)

Since The Merge (September 2022), Ethereum runs as **two cooperating clients**:

```
┌──────────────────────────────────────────────────────┐
│            CONSENSUS LAYER (CL)                       │
│  Lighthouse / Prysm / Teku / Nimbus / Lodestar       │
│                                                       │
│  • Validator management (deposit, activate, exit)     │
│  • Leader election (RANDAO + shuffling)               │
│  • Attestations (votes on canonical head)             │
│  • Finality (Casper FFG: justified → finalized)       │
│  • Slashing (double-vote, surround-vote)              │
│  • BLS12-381 signatures (ALL crypto here)             │
│  • Slot/epoch timing (12s / 384s)                     │
└────────────────────┬─────────────────────────────────┘
                     │ Engine API (JSON-RPC)
                     │   engine_newPayloadV*
                     │   engine_forkchoiceUpdatedV*
                     │   engine_getPayloadV*
┌────────────────────▼─────────────────────────────────┐
│            EXECUTION LAYER (EL)                        │
│  reth / geth / Nethermind / Besu / Erigon             │
│                                                       │
│  • EVM execution (opcodes, precompiles)               │
│  • Transaction pool                                   │
│  • State management (accounts, storage, trie)         │
│  • Block body validation (gas, receipts)              │
│  • P2P block/tx propagation                           │
│  • ECDSA/secp256k1 signatures (tx layer)              │
└──────────────────────────────────────────────────────┘
```

The EL is a **passive follower** — it never decides which block is canonical. The CL drives all consensus decisions and communicates them via Engine API.

### 1.2 Engine API — The Bridge

| Method | Direction | Purpose |
|--------|-----------|---------|
| `engine_forkchoiceUpdatedV*` | CL → EL | Tell EL the canonical head + trigger payload building |
| `engine_newPayloadV*` | CL → EL | Send execution payload for validation |
| `engine_getPayloadV*` | CL ← EL | Retrieve built payload for proposal |

The EL trusts whatever `forkchoiceUpdated` the CL sends. It has no ability to reject consensus decisions.

### 1.3 BLS12-381 — The Cryptographic Foundation of PoS

The Beacon Chain uses **BLS12-381** (Boneh-Lynn-Shacham over the BLS12-381 curve) for all validator operations:

| Operation | BLS Usage | Size |
|-----------|-----------|------|
| Validator identity | BLS public key | 48 bytes |
| Attestation signature | BLS sign(AttestationData) | 96 bytes |
| Aggregate attestation | BLS aggregate(sig₁, sig₂, ..., sigₙ) | 96 bytes (!) |
| Block proposal | BLS sign(BeaconBlock) | 96 bytes |
| RANDAO reveal | BLS sign(epoch) | 96 bytes |
| Sync committee | BLS aggregate for light clients | 96 bytes |

**Critical property: BLS aggregation.** Hundreds of individual 96-byte signatures can be combined into a single 96-byte aggregate signature that can be verified against the aggregate public key. This allows Ethereum to support ~900,000 validators without blocks becoming prohibitively large.

### 1.4 Slot and Epoch Timing

- **Slot:** 12 seconds. At most one block per slot.
- **Epoch:** 32 slots = 384 seconds (~6.4 minutes).
- Each epoch, all active validators are shuffled into 32 committees (one per slot).
- Each validator attests exactly once per epoch.
- A proposer is pseudo-randomly selected for each slot (weighted by effective balance).

### 1.5 RANDAO — Leader Election

```
Each slot, the proposer reveals:
  RANDAO_reveal = BLS_sign(sk_validator, epoch)

The reveal is mixed into the beacon state:
  randao_mixes[epoch % 65536] ^= hash(RANDAO_reveal)

Accumulated randomness determines:
  1. Committee shuffling (which validators attest in which slot)
  2. Proposer selection (who builds the next block)
```

RANDAO depends on BLS's **deterministic signing** — the same key always produces the same signature for the same message, preventing manipulation.

### 1.6 Casper FFG — Finality

- Two-phase commit on epoch boundaries (checkpoints).
- **Justified:** ≥2/3 of total active stake attests to a checkpoint.
- **Finalized:** A justified checkpoint whose child epoch is also justified.
- Finality time: typically 2 epochs (~12.8 minutes).
- **Guarantee:** A finalized block cannot be reverted without ≥1/3 of total stake being slashed (economic finality).

### 1.7 Validator Lifecycle

```
Deposit (32 ETH) → Pending Queue → Active → Attesting/Proposing → Exit → Withdrawal
                                      │
                                      └─ Slashing (if misbehaving)
                                         • Double voting (same target epoch)
                                         • Surround voting (conflicting source/target)
                                         • Double proposal (same slot)
                                         Penalty: 1/32 to 100% of stake
```

---

## 2. Why Ethereum PoS Cannot Be Directly Made Post-Quantum

### 2.1 The Core Problem: BLS12-381 is Broken by Shor's Algorithm

BLS12-381 is an **elliptic curve** — it relies on the hardness of the Discrete Logarithm Problem (DLP) over pairing-friendly curves. Shor's algorithm solves DLP in polynomial time on a quantum computer.

| CL Component | Cryptographic Primitive | Quantum Vulnerability |
|--------------|------------------------|----------------------|
| Validator keys | BLS12-381 keypair | **Broken** (Shor) |
| Attestations | BLS signature | **Broken** (Shor) |
| Aggregation | BLS aggregate | **Broken** (Shor) |
| RANDAO | BLS deterministic sign | **Broken** (Shor) |
| Sync committees | BLS aggregate | **Broken** (Shor) |

A quantum attacker could:
1. **Forge validator signatures** — produce valid attestations for any validator
2. **Steal validator keys** — derive private keys from public keys
3. **Break finality** — create conflicting finalized checkpoints
4. **Manipulate RANDAO** — predict or bias leader election

### 2.2 No PQ Signature Scheme Has BLS Aggregation

The killer feature of BLS is **non-interactive aggregation**: N signatures compress to 1 signature of constant size (96 bytes regardless of N). No known post-quantum signature scheme has this property:

| Scheme | Sig Size | Aggregatable? | Status |
|--------|----------|---------------|--------|
| BLS12-381 | 96 B | **Yes** (constant-size) | Broken by Shor |
| ML-DSA-65 (Dilithium) | 3,309 B | No | PQ-safe |
| FALCON-512 | 666 B | No | PQ-safe |
| SPHINCS+-128s | 7,856 B | No | PQ-safe |

Without aggregation, carrying attestations from ~900K validators would require:
- BLS (current): ~96 bytes per slot (aggregated)
- ML-DSA-65: ~3,309 × validators_per_committee ≈ **1.06 MB per slot** (for 320 attesters)
- This would break the 12-second slot assumption and explode beacon block sizes.

### 2.3 The CL is a Separate Codebase

The Consensus Layer is implemented in **completely different codebases**:

| CL Client | Language | LOC (approx) |
|-----------|----------|--------------|
| Lighthouse | Rust | ~200,000 |
| Prysm | Go | ~300,000 |
| Teku | Java | ~200,000 |
| Nimbus | Nim | ~100,000 |
| Lodestar | TypeScript | ~150,000 |

Forking any of these and replacing BLS with ML-DSA would require:
1. Replacing all BLS signature operations
2. Redesigning attestation aggregation (fundamental protocol change)
3. Redesigning RANDAO (cannot use deterministic signing)
4. Adjusting slot timing (larger messages, no aggregation)
5. Redesigning the P2P gossip layer (attestation subnets)
6. Rewriting the fork choice rule (adapted for new attestation format)

**This is equivalent to designing a new consensus protocol from scratch** while inheriting the complexity of Ethereum's beacon chain. Infeasible for this project's scope.

### 2.4 Our EL Is PQ-Safe, But the CL Is Not

This project (PostQuantumEVM) has successfully made the **Execution Layer** post-quantum:

| Layer | Classical Crypto | Our PQ Replacement | Status |
|-------|-----------------|-------------------|--------|
| EL: Transaction signatures | ECDSA/secp256k1 | ML-DSA-65 | **Done** |
| EL: Address derivation | keccak256(pk) | shake256(pk) | **Done** |
| EL: Precompiles | ecrecover, BN254, BLS12-381 | Disabled + ML-DSA verify | **Done** |
| CL: Validator signatures | BLS12-381 | ??? | **Not feasible** |
| CL: Aggregation | BLS aggregate | No PQ equivalent exists | **Unsolvable** |
| CL: RANDAO | BLS deterministic sign | ??? | **Not feasible** |

The EL alone cannot provide consensus. Without a PQ-safe CL, a quantum attacker could control the consensus layer and force the (PQ-safe) EL to accept malicious blocks.

---

## 3. Our Solution: Proof of Authority with ML-DSA-65

### 3.1 Why PoA

Given the impossibility of porting Ethereum's PoS to a PQ setting without a complete CL redesign, we adopt **Proof of Authority (PoA)** — a consensus mechanism where a fixed set of authorized validators take turns producing blocks, signing with ML-DSA-65.

| Factor | PoS (Ethereum) | PoA (PQ-EVM) |
|--------|---------------|--------------|
| Security model | Economic (stake) | Identity (authorized keys) |
| Signature scheme | BLS12-381 (broken) | ML-DSA-65 (PQ-safe) |
| Aggregation needed | Yes (900K validators) | No (3-10 validators) |
| Finality | Probabilistic (2 epochs) | Deterministic (N/2+1 sigs) |
| Decentralization | High (~900K validators) | Low (permissioned set) |
| Implementation complexity | Extreme (full CL) | Moderate (embedded engine) |
| PQ-safe end-to-end | No (CL uses BLS) | **Yes** (ML-DSA everywhere) |

### 3.2 Design Overview

```
┌───────────────────────────────────────────────────────┐
│                PQ-EVM PoA Consensus                    │
│                                                       │
│  Validator Set: [(addr₁, pk₁), (addr₂, pk₂), ...]    │
│  All keys are ML-DSA-65 (post-quantum safe)           │
│                                                       │
│  Block Production: Round-Robin                        │
│    slot N → validator[N % num_validators]             │
│                                                       │
│  Block Sealing:                                       │
│    extra_data = ML-DSA-65-sign(sk, block_header_hash) │
│                                                       │
│  Finality: Immediate (single signer = authority)      │
│                                                       │
│  Fault Tolerance:                                     │
│    If validator misses slot → timeout → next in line  │
└───────────────────────────────────────────────────────┘
```

### 3.3 Protocol Specification

#### Validator Set

Defined in the genesis configuration:

```json
{
  "poa_validators": [
    {
      "address": "0x...",
      "public_key": "<1952-byte ML-DSA-65 verifying key, hex-encoded>"
    }
  ]
}
```

The validator set is **static** (defined at genesis). A future extension could add a validator registry contract for dynamic membership.

#### Slot Assignment

```
slot_number = block_number
proposer_index = slot_number % num_validators
proposer = validators[proposer_index]
```

Simple round-robin rotation. Each validator gets equal proposal rights.

#### Block Sealing

The proposer signs the block header (excluding the signature field) with their ML-DSA-65 key:

```
header_hash = shake256(RLP(header_without_seal))
seal = ml_dsa_65_sign(proposer_sk, header_hash)
header.extra_data = seal  (3309 bytes)
```

#### Block Verification

Any node receiving a block verifies:

```
1. Determine expected proposer: validators[block.number % N]
2. Extract seal from header.extra_data (3309 bytes)
3. Compute header_hash = shake256(RLP(header_without_seal))
4. Verify: ml_dsa_65_verify(proposer.pk, header_hash, seal)
5. If invalid → reject block
```

#### Missed Slots / Fault Tolerance

```
If current proposer doesn't produce within slot_time:
  → Next validator in rotation takes over
  → Block.number still increments (no empty slots in chain)
  → Tracks consecutive misses per validator (future: eviction)
```

#### Block Time

- **Slot time:** 5 seconds (configurable)
- **No empty slots:** If a validator misses, the next takes over immediately
- **No epoch structure:** Every block is immediately final (single authority per block)

### 3.4 Security Properties

| Property | Guarantee |
|----------|-----------|
| **Post-quantum safety** | All signing uses ML-DSA-65 (NIST FIPS 204, Category 3) |
| **Liveness** | Tolerates up to N-1 offline validators (round-robin skip) |
| **Safety** | No forks as long as ≤1 validator produces per slot |
| **Finality** | Immediate (1 block = final, no reorgs) |
| **Sybil resistance** | Permissioned set (not open membership) |

### 3.5 Trade-offs vs Ethereum PoS

| Aspect | We Lose | We Gain |
|--------|---------|---------|
| Decentralization | Open validator set | PQ-safe end-to-end |
| Economic security | Stake-based penalties | Simpler implementation |
| Validator count | Thousands | Manageable (3-20) |
| Aggregation | N/A (not needed) | No BLS dependency |
| Finality time | 12.8 min → 5 sec | Instant finality |
| Light clients | Sync committees | Direct header verification |

### 3.6 Integration with reth

The PoA engine integrates by **driving the Engine API internally**, replacing the external CL:

```
┌─────────────────────────────────────────┐
│   PQ-PoA Engine (embedded in pq-reth)   │
│                                         │
│   • Monitors slot timer (5s)            │
│   • Checks if we are the proposer       │
│   • Calls engine_getPayload → build     │
│   • Signs header with ML-DSA-65         │
│   • Calls engine_newPayload → validate  │
│   • Calls forkchoiceUpdated → finalize  │
└────────────────────┬────────────────────┘
                     │ (internal Engine API calls)
┌────────────────────▼────────────────────┐
│   PQ-EVM Execution Layer (reth)         │
│   (unchanged — same as before)          │
└─────────────────────────────────────────┘
```

This means the EL code (EVM, pool, state) remains unchanged. The PoA logic is a thin layer on top that drives block production.

---

## 4. Future Work: Toward PQ Proof of Stake

A full PQ PoS system would require research advances in:

1. **Lattice-based aggregate signatures** — active research area, no practical scheme yet
2. **PQ-safe VRFs** — for unbiasable leader election (replacing RANDAO)
3. **Compact PQ attestations** — achieving scalability with large signature sizes
4. **PQ slashing proofs** — cryptographic evidence of misbehavior without BLS

Until these primitives exist and are standardized, PoA with ML-DSA-65 provides a pragmatic, fully post-quantum consensus mechanism suitable for permissioned networks, testnets, and enterprise deployments.

---

## 5. Comparison Summary

```
Ethereum PoS                          PQ-EVM PoA
─────────────                         ──────────
BLS12-381 (BROKEN by Shor)     →     ML-DSA-65 (PQ-safe, FIPS 204)
~900K validators               →     3-20 authorized validators
96B aggregated attestations    →     3,309B individual signatures
12.8 min finality              →     5s instant finality
Separate CL client required    →     Embedded in single binary
Economic security (32 ETH)     →     Identity-based trust
Open permissionless            →     Permissioned validator set
```

**Conclusion:** The pivot from PoS to PoA is not a simplification by choice but a **necessity** driven by the fundamental incompatibility of BLS12-381 aggregation with post-quantum cryptography. No known PQ signature scheme can replicate BLS's aggregation property, making Ethereum-style PoS architecturally impossible in a post-quantum setting with current cryptographic knowledge.
