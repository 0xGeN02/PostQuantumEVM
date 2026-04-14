"""
Quantum ECDLP solver using Shor's algorithm
===========================================

Applied to the secp256k1 curve used in Ethereum

Finds k such that Q = k · G

k: private key (integer)
Q: public key (point on the curve)
G: generator point of the curve (fixed point on the curve)

Shor-based ECDLP algorithm:

f(a, b) = a·G + b·Q = (a + b·k)·G  has period lattice ⟨(N,0), (k,-1)⟩

Steps:
    1. Evaluate f(a,b) = a·G + b·Q on a uniform superposition |a⟩|b⟩
    2. Prepare the coset state (1/√N) ∑_b |(p₀ - b·k) mod N⟩ |b⟩
    3. Apply 2D QFT modulo N on |a⟩ |b⟩
    4. Measure (s, t) → integers in [0, N)
    5. Recover k ≡ s⁻¹ · t mod N when gcd(s, N) = 1
"""
import numpy as np
from math import gcd
from qiskit import QuantumCircuit

from lib.secp256k1_toy import INFINITY, scalar_mult, G, N

# ── True order of G (smallest n ≥ 1 s.t. n·G = ∞) ──────────────────────────
# N from secp256k1_toy is the curve point count; the generator's cyclic
# subgroup may be a proper divisor.
ORDER: int = next(i for i in range(1, N + 1) if scalar_mult(i, G) is INFINITY)

# group_points[i] = i·G for i ∈ [0, ORDER)
_group_points: list = [INFINITY] + [scalar_mult(i, G) for i in range(1, ORDER)]


def _point_to_idx(pt) -> int:
    """Discrete-log index i such that i·G == pt (mod ORDER)."""
    if pt is INFINITY:
        return 0
    return _group_points.index(pt)


# ── Coset state ──────────────────────────────────────────────────────────────
def build_coset_state(k: int, p0: int) -> np.ndarray:
    """
    Coset state after oracle evaluation and ancilla measurement:

        |ψ⟩ = (1/√ORDER) ∑_{b=0}^{ORDER-1} |(p₀ - b·k) mod ORDER⟩_a |b⟩_b

    Returns an ORDER × ORDER complex matrix  ψ[b, a].
    """
    psi = np.zeros((ORDER, ORDER), dtype=complex)
    for b in range(ORDER):
        a = (p0 - b * k) % ORDER
        psi[b, a] = 1.0
    psi /= np.linalg.norm(psi)
    return psi


# ── Exact modular DFT (simulates the ideal QFT_N) ───────────────────────────
def modular_dft_2d(psi: np.ndarray) -> np.ndarray:
    """
    2D DFT of size ORDER × ORDER (exact modular QFT).

    F[t, s] = (1/ORDER) ∑_{b,a} ψ[b,a] · ω^{-(s·a + t·b)}

    where ω = exp(2πi / ORDER).
    """
    return np.fft.fft2(psi) / ORDER


def sample_from_dft(F: np.ndarray, shots: int) -> dict[tuple[int, int], int]:
    """
    Sample (s, t) pairs from the probability distribution |F[t, s]|².
    Returns a dict  {(s, t): count}.
    """
    probs = np.abs(F) ** 2
    probs /= probs.sum()  # ensure normalization
    flat_probs = probs.flatten()

    rng = np.random.default_rng()
    indices = rng.choice(len(flat_probs), size=shots, p=flat_probs)

    counts: dict[tuple[int, int], int] = {}
    for idx in indices:
        t, s = divmod(int(idx), ORDER)
        key = (s, t)
        counts[key] = counts.get(key, 0) + 1
    return counts


# ── Classical post-processing ────────────────────────────────────────────────
def decode_k(counts: dict[tuple[int, int], int]) -> int | None:
    """
    For each measured (s, t) pair, recover k = s⁻¹ · t mod ORDER.
    Returns the most-voted candidate, or None.
    """
    votes: dict[int, int] = {}

    for (s, t), freq in counts.items():
        if s == 0 or t == 0:
            continue
        if gcd(s, ORDER) != 1:
            continue
        k_cand = (pow(s, -1, ORDER) * t) % ORDER
        if k_cand == 0:
            continue
        votes[k_cand] = votes.get(k_cand, 0) + freq

    if not votes:
        return None
    return max(votes, key=votes.get)


# ── Qiskit circuit (for visualization / pedagogical purposes) ────────────────
def _apply_iqft(qc: QuantumCircuit, qubits: list[int]) -> None:
    """Inverse QFT using H, CP, and SWAP gates."""
    n = len(qubits)
    for j in range(n - 1, -1, -1):
        for k in range(n - 1, j, -1):
            qc.cp(-np.pi / 2 ** (k - j), qubits[j], qubits[k])
        qc.h(qubits[j])
    for i in range(n // 2):
        qc.swap(qubits[i], qubits[n - 1 - i])


def build_circuit(k: int, p0: int) -> QuantumCircuit:
    """
    Build a Qiskit circuit representing the Shor ECDLP attack.

    Uses ceil(log2(ORDER)) qubits per register.  The circuit is useful for
    visualization; the actual simulation uses the exact modular DFT above
    since the base-2 QFT introduces spectral leakage when ORDER is not
    a power of 2.
    """
    n_q = int(np.ceil(np.log2(ORDER)))
    D = 1 << n_q

    # Embed the ORDER × ORDER coset state into a D × D Hilbert space
    psi_flat = np.zeros(D * D, dtype=complex)
    for b in range(ORDER):
        a = (p0 - b * k) % ORDER
        psi_flat[a + D * b] = 1.0
    psi_flat /= np.linalg.norm(psi_flat)

    qc = QuantumCircuit(2 * n_q, 2 * n_q)
    qc.initialize(psi_flat, range(2 * n_q))
    qc.barrier()

    _apply_iqft(qc, list(range(n_q)))            # QFT† on register a
    _apply_iqft(qc, list(range(n_q, 2 * n_q)))   # QFT† on register b
    qc.barrier()

    qc.measure(range(2 * n_q), range(2 * n_q))
    return qc


# ── Public API ───────────────────────────────────────────────────────────────
def shor_ecdlp(
    Q: tuple,
    shots: int = 4096,
) -> tuple[int | None, dict[tuple[int, int], int], np.ndarray]:
    """
    Recover the private key k from a public key Q = k·G on the toy secp256k1
    curve using Shor's ECDLP algorithm.

    Parameters
    ----------
    Q     : public key (point on the curve)
    shots : number of measurement samples

    Returns
    -------
    k_recovered : the recovered private key, or None
    counts      : measurement histogram {(s, t): frequency}
    probs       : ORDER × ORDER probability matrix |F[t, s]|²
    """
    k_true = _point_to_idx(Q)
    if k_true == 0:
        raise ValueError("Q is the point at infinity (trivial case)")

    p0 = np.random.randint(0, ORDER)
    psi = build_coset_state(k_true, p0)
    F = modular_dft_2d(psi)
    probs = np.abs(F) ** 2

    counts = sample_from_dft(F, shots)
    k_recovered = decode_k(counts)

    return k_recovered, counts, probs
