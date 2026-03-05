"""
Grover's Algorithm for SHA3 Preimage Search
============================================

Any cryptographic hash function can be represented as:

    H(x) = Y

H => Hash function (e.g., SHA3)
x => Input string (preimage)
Y => Output hash

For SHA-3 (NIST's keccak implementation) we specify the variant:

    Trunc_n(SHA3_k(x)) = Y

k => SHA3 variant k ∈ {224, 256, 384, 512}
x => Input string (preimage)
Y => Target output (n bits)
n => Number of bits to match in the output

Complexity SHA3:
    - Classical: O(2^n)
    - Quantum: O(2^(n/2))
    
Example for n=8 and k=256:

SHA3-256(x)[:8] == Y
    - Classical: O(2^8) = 256
    - Quantum: O(2^(8/2)) = O(2^4) = 16

Simulation limits (AerSimulator local):
  n ≤ 16 bits: feasible on a laptop (~16 GB RAM)
  n = 32 bits: requires real QPU or HPC cluster
  n = 64 bits: ~2^32 Grover iterations on a QPU

Oracle note:
  A real quantum attack on SHA3 requires implementing the Keccak-f[1600]
  permutation as a reversible quantum circuit (~2.4B Toffoli gates for
  full SHA3-256 — currently beyond available hardware).
  This implementation uses a direct phase oracle for simulation purposes.

Note: the complexity depends on n (matched bits), not on k (hash output size).
"""

import hashlib
import numpy as np
from qiskit import QuantumCircuit, Aer, transpile, assemble, execute

