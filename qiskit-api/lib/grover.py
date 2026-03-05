import numpy as np
from qiskit import QuantumCircuit
from qiskit_aer import AerSimulator

from lib.keccak_toy import build_lookup

# Build the full lookup table: all 16 possible inputs → 4-bit hash
lookup = build_lookup()
print("Toy Keccak lookup table:")
for inp, out in lookup.items():
    print(f"  {''.join(map(str,inp))} -> {''.join(map(str,out))}")
        
def make_phase_oracle(target_hash: tuple, n: int = 4) -> QuantumCircuit:
    """
    Phase oracle U_f for toy Keccak.
    Flips phase of |x> if keccak_toy(x) == target_hash.
    Uses a direct encoding: X-MCZ-X pattern for each solution.
    """
    qc = QuantumCircuit(n, name="U_f (Keccak oracle)")
    
    # Find all preimages of the target hash
    solutions = [inp for inp, out in lookup.items() if out == target_hash]
    if not solutions:
        print(f"No preimage found for target {target_hash}")
        return qc

    for sol in solutions:
        # Flip qubits where solution has 0 (so MCZ fires on all-|1> = our target)
        for i, bit in enumerate(sol):
            if bit == 0:
                qc.x(i)
        # Multi-controlled Z: flips phase of |1111>
        qc.h(n - 1)
        qc.mcx(list(range(n - 1)), n - 1)  # Toffoli generalised
        qc.h(n - 1)
        # Undo X flips
        for i, bit in enumerate(sol):
            if bit == 0:
                qc.x(i)

    return qc

# Diffuser (Grover's inversion about the mean)
def make_diffuser(n: int) -> QuantumCircuit:
    """
    U_s = 2|+><+| - I  (inversion about the uniform superposition)
    """
    qc = QuantumCircuit(n, name="Diffuser")
    qc.h(range(n))
    qc.x(range(n))
    qc.h(n - 1)
    qc.mcx(list(range(n - 1)), n - 1)
    qc.h(n - 1)
    qc.x(range(n))
    qc.h(range(n))
    return qc

def grover_keccak(target_hash: tuple, n: int = 4, shots: int = 1024) -> tuple:
    """
    Run Grover's algorithm to find x such that keccak_toy(x) == target_hash.

    Parameters
    ----------
    target_hash : tuple of 4 ints (bits)
    n           : number of input qubits (= 4 for toy Keccak)
    shots       : number of measurement shots

    Returns
    -------
    (circuit, counts, solutions)
    """
    # M = number of solutions — k_opt depends on M, not just N
    solutions_found = [inp for inp, out in lookup.items() if out == target_hash]
    M = len(solutions_found)
    n_iter = max(1, int(np.floor(np.pi / 4 * np.sqrt(2**n / M))))
    print(f"Target hash : {''.join(map(str, target_hash))}")
    print(f"N           : {2**n} states,  M = {M} solution(s)")
    print(f"Iterations  : {n_iter}  (optimal = ⌊π/4 · √(N/M)⌋)")

    oracle   = make_phase_oracle(target_hash, n)
    diffuser = make_diffuser(n)

    qc = QuantumCircuit(n, n)
    # Uniform superposition
    qc.h(range(n))
    qc.barrier()
    # Grover iterations
    for _ in range(n_iter):
        qc.compose(oracle,   inplace=True)
        qc.compose(diffuser, inplace=True)
        qc.barrier()
    # Measure
    qc.measure(range(n), range(n))

    sim    = AerSimulator()
    result = sim.run(qc, shots=shots).result()
    counts = result.get_counts()
    return qc, counts, solutions_found