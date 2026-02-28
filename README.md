# PostQuantum EVM

## A Post-Quantum Cryptography EVM Blockchain

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/Python_3.13-3776AB?logo=python&logoColor=white&style=for-the-badge)](https://python.org)
[![Solidity](https://img.shields.io/badge/Solidity-503D8B?style=for-the-badge&logo=ethereum&logoColor=white)](https://docs.soliditylang.org/en/latest/)
[![License: MIT](https://img.shields.io/badge/License-APACHE_2.0-yellow.svg?style=for-the-badge)](LICENSE)

> [!IMPORTANT]
> **Preparing blockchain systems for the post-quantum era.**  
> **This project demonstrates a forward-looking approach to cryptographic security.**

</div>

---

## Overview

The **Post-Quantum EVM Blockchain** is a forward-looking project that aims to adapt the Ethereum Virtual Machine (EVM) to the post-quantum era. By forking the Reth client, this project integrates post-quantum cryptographic primitives and introduces a comprehensive ecosystem to ensure the security and scalability of blockchain systems in the face of quantum computing threats.

## Arquitecture

![arquitecture](./docs/architecture.png)

### Key Components

| Component                          | Description                                                                 |
|------------------------------------|-----------------------------------------------------------------------------|
| **Post-Quantum Cryptographic Library** | Rust-based library implementing NIST-standardized post-quantum algorithms. |
| **Post-Quantum Wallets**           | Secure wallets for key management and transaction signing.                 |
| **Qiskit API**                     | Python-based API for simulating quantum attacks and testing resilience.    |
| **Post-Quantum Reth Client**       | Forked Reth client with post-quantum cryptographic primitives.             |
| **EIP-Compatible Precompiled Contracts** | Custom precompiled contracts for post-quantum cryptographic operations.    |

This project is designed to provide a robust foundation for building secure and scalable blockchain systems that are resistant to quantum computing threats, while maintaining compatibility with existing EVM-based smart contracts.

---

## Roadmap

### Phase 1: Post-Quantum Cryptographic Library

- Develop a Rust-based library implementing NIST-standardized post-quantum algorithms:
  - CRYSTALS-Dilithium for digital signatures.
  - CRYSTALS-Kyber for key encapsulation.
  - SPHINCS+ for hash-based signatures.
- Benchmark post-quantum algorithms against classical cryptographic alternatives.

### Phase 2: Post-Quantum Wallets

- Design and implement post-quantum wallets for secure key management and transaction signing.
- Integrate the post-quantum cryptographic library into wallet operations.
- Ensure compatibility with the modified Reth client and EVM.

### Phase 3: Qiskit API for Quantum Attack Simulation

- Develop a Python-based API using Qiskit to simulate quantum attacks:
  - Implement Grover's algorithm to test hash function vulnerabilities.
  - Implement Shor's algorithm to demonstrate the breaking of classical cryptographic primitives.
- Provide endpoints for testing blockchain resilience against quantum attacks.

### Phase 4: Post-Quantum Reth Client

- Fork the Reth client and integrate the post-quantum cryptographic library.
- Modify the Reth client to support post-quantum cryptographic primitives for:
  - Transaction validation.
  - Block verification.
  - Consensus mechanisms.
- Ensure compatibility with existing EVM-based smart contracts.

### Phase 5: EIP-Compatible Precompiled Contracts

- Define EIPs for post-quantum cryptographic operations in the EVM.
- Implement precompiled contracts for:
  - Post-quantum key derivation and transaction signing.
  - Post-quantum hash functions (e.g., BLAKE3, SHA3).
  - Post-quantum digital signatures (e.g., Dilithium).
- Adapt opcode semantics and gas metering for post-quantum operations.

### Phase 6: Post-Quantum Blockchain Implementation

- Deploy the post-quantum blockchain using the modified Reth client.
- Validate the blockchain's resistance to quantum attacks.
- Provide detailed documentation and examples for developers to build post-quantum smart contracts.

### Phase 7: Interactive CLI and Developer Tools

- Develop a command-line interface (CLI) to:
  - Initialize and manage the post-quantum blockchain.
  - Create and manage post-quantum wallets.
  - Simulate quantum attacks using the Qiskit API.
  - Deploy and interact with post-quantum smart contracts.
- Provide detailed documentation and examples for CLI usage.

---

## Why This Matters

The rise of quantum computing poses a significant threat to classical cryptographic systems, including those used in blockchain technologies like Ethereum. Algorithms such as **ECDSA** and **SHA-256**, which are foundational to current blockchain security, are vulnerable to quantum attacks like **Shor's algorithm** and **Grover's algorithm**. This could render existing blockchain systems insecure within the next decade.

The **Post-Quantum EVM Blockchain** addresses these challenges by:

- **Post-Quantum Cryptography**: Integrating NIST-standardized post-quantum algorithms (e.g., CRYSTALS-Dilithium, CRYSTALS-Kyber, SPHINCS+) to replace vulnerable classical cryptographic primitives.
- **EVM Compatibility**: Ensuring that the blockchain remains compatible with existing Ethereum smart contracts while introducing post-quantum security.
- **Precompiled Contracts**: Providing EIP-compatible precompiled contracts for post-quantum cryptographic operations, enabling developers to build quantum-resistant smart contracts.
- **Post-Quantum Wallets**: Offering secure wallets for key management and transaction signing using post-quantum cryptography.
- **Quantum Attack Simulation**: Leveraging the Qiskit API to simulate quantum attacks and validate the blockchain's resilience.

This project is a critical step toward ensuring the long-term security and scalability of blockchain systems in the quantum era, enabling developers and organizations to future-proof their decentralized applications and infrastructure.

---

## Author

**0xGeN02**  
Building secure and scalable systems at the intersection of cryptography and distributed systems.  
[GitHub Profile](https://github.com/0xGeN02)

---

## Published Paper

The research paper detailing the design and implementation of the Post-Quantum EVM Blockchain will be published soon. Stay tuned for updates!

---

<div align="center">

*Preparing blockchain systems for the quantum era.*

</div>
