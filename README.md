# PQC-Blockchain

## A Post-Quantum Cryptography EVM Blockchain

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/Python_3.13-3776AB?logo=python&logoColor=white&style=for-the-badge)](https://python.org)
[![Solidity](https://img.shields.io/badge/Solidity-503D8B?style=for-the-badge&logo=ethereum&logoColor=white)](https://docs.soliditylang.org/en/latest/)
[![License: MIT](https://img.shields.io/badge/License-APACHE_2.0-yellow.svg?style=for-the-badge)](LICENSE)

> [!NOTE]
> **Preparing blockchain systems for the post-quantum era.** </br>
> **This project demonstrates a forward-looking approach to cryptographic security.**

---

## Overview

Current blockchain systems, such as Bitcoin and Ethereum, rely on cryptographic algorithms like **ECDSA** and **SHA-256**, which are vulnerable to quantum computers running **Shor's algorithm**. With the advent of quantum computing, these algorithms may become obsolete within the next decade. Recognizing this challenge, NIST has standardized post-quantum cryptographic algorithms to address these vulnerabilities.

**PQC-Blockchain** is a blockchain implementation in Rust that integrates post-quantum cryptographic primitives to ensure long-term security. This project highlights:

- Expertise in cryptographic principles and their practical application.
- Systems-level programming in Rust, emphasizing safety and performance.
- A modular and extensible architecture designed for real-world use cases.

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                      CLI Layer                       │
│               (User Interface & Commands)            │
└──────────────────────┬──────────────────────────────┘
                       │
         ┌─────────────┴─────────────┐
         ▼                           ▼
┌─────────────────┐       ┌───────────────────┐
│ Blockchain Core │◄─────►│  PQC Algorithms   │
│                 │       │                   │
│  • Blocks       │       │  • Lattice-based  │
│  • Transactions │       │  • Hash-based     │
│  • Consensus    │       │  • Code-based     │
│  • Merkle Trees │       │  • NIST Standards │
│  • Chain State  │       │                   │
└─────────────────┘       └───────────────────┘
```

---

## Roadmap

### Phase 1: Fundamentals and Demonstrations

- Implement Grover's and Shor's algorithms to demonstrate vulnerabilities in classical cryptography.
- Develop a basic blockchain using SHA-256 and ECDSA.
- Use quantum simulators (e.g., Qiskit, QVM) to break the blockchain's cryptographic primitives.

### Phase 2: Analysis and Design of Solutions

- Analyze existing consensus protocols (e.g., Proof-of-Work, Proof-of-Stake) and their resistance to quantum attacks.
- Evaluate the necessity of a quantum consensus protocol.
- Design a post-quantum solution for the blockchain.

### Phase 3: Implementation of Post-Quantum Algorithms

- Integrate CRYSTALS-Dilithium for digital signatures.
- Integrate CRYSTALS-Kyber for key encapsulation.
- Implement SPHINCS+ for hash-based signatures.
- Benchmark classical cryptographic algorithms against post-quantum alternatives.

### Phase 4: Post-Quantum Blockchain Implementation

- Develop a post-quantum blockchain based on the designed solution.
- Validate the blockchain's resistance to quantum attacks.
- Provide a detailed explanation of why the solution is secure.

### Phase 5: Advanced Extensions

- Implement pseudologic or pseudosmartcontracts for inter-node operations.
- Research and explain rollups and zk-proofs (zero-knowledge proofs) that are quantum-resistant.
- Integrate examples of zk-proofs and rollups into the blockchain.

### Phase 6: Interactive CLI

- Develop a command-line interface (CLI) to:
  - Launch the blockchain.
  - Simulate quantum attacks (Grover's, Shor's).
  - Test the post-quantum blockchain.
  - Execute pseudologic or smart contracts.
  - Demonstrate zk-proofs and rollups.
- Provide detailed documentation for CLI usage.

---

## Why This Matters

```
Classical Blockchain              PQC-Blockchain
─────────────────────             ──────────────────────
ECDSA signatures         →       CRYSTALS-Dilithium (lattice-based)
ECDH key exchange        →       CRYSTALS-Kyber (lattice-based)
Vulnerable to Shor's     →       Quantum-resistant by design
algorithm in ~2030s               NIST standardized (2024)
```

The **NIST Post-Quantum Cryptography Standardization** finalized its first standards in 2024. This project implements these standards in a blockchain context, ensuring long-term security against quantum threats.

---

## Contact

Building secure and scalable systems at the intersection of cryptography and distributed systems.

[0xGeN02](https://github.com/0xGeN02)

---

<div align="center">

*Preparing blockchain systems for the quantum era.*

</div>
