# PQ-EVM Solidity Contracts

Smart contracts designed for the Post-Quantum EVM (chain_id=20561).

## Contracts

| Contract | Description |
|----------|-------------|
| `PQVerify.sol` | Library to call the ML-DSA-65 verify precompile at `0x0100` |
| `PQHash.sol` | Helper for the PQHASH opcode (SHAKE-256, opcode `0x21`) |
| `PQMultiSig.sol` | M-of-N multi-signature wallet using ML-DSA-65 |
| `PQAccessControl.sol` | Role-based access control with PQ signature verification |

## Tests

`test/PQGas.t.sol` measures gas consumption of PQ operations. Requires a running pq-reth node:

```bash
# Start the PQ node (in another terminal)
cargo run --bin pq-reth -- node --dev --dev.block-time 5s --chain pq-reth/bin/pq-reth/genesis.json

# Run gas tests
forge test --gas-report --fork-url http://localhost:8545 -vv
```

## Setup

```bash
cd contracts
forge install   # fetches forge-std submodule
forge build
```

## Dependencies

- `forge-std` — Foundry standard library (git submodule)

## Notes

- `ecrecover()` is **disabled** on this chain (returns error). All ECDSA-based patterns are broken.
- Use `PQVerify.verify(msgHash, sig, pk)` as the post-quantum replacement.
- The PQHASH opcode (`0x21`) provides native SHAKE-256 at the same gas cost as KECCAK256 (`0x20`).
