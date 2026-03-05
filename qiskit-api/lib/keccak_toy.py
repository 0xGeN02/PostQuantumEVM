"""
Toy Keccak implementation for educational and quantum simulation purposes.

Parameters
----------
B        : state size = 8 bits
R        : rate      = 4 bits
C        : capacity  = 4 bits  (B - R)
L        : output    = 4 bits
N_ROUNDS : rounds    = 2

Simplifications vs real Keccak-f[1600]:
  - 1D state instead of 5×5×64 lanes
  - rho: cyclic shift by 1 (not offset table)
  - theta and pi steps omitted (both are linear; chi dominates oracle cost)
  - 2 rounds instead of 24
  - Fixed round constants instead of LFSR-derived

These simplifications are acceptable for demonstrating the Grover oracle
construction because chi — the only non-linear step — is identical in
structure to the real Keccak chi: A_i ^= (~A_{i+1}) & A_{i+2}.
"""

B: int = 8
R: int = 4
C: int = B - R
L: int = 4
N_ROUNDS: int = 2


def rho(state: list) -> list:
    """Cyclic left-shift of the state by 1 position (simplified ρ)."""
    return state[1:] + state[:1]


def chi(state: list) -> list:
    """
    Non-linear mixing layer (χ).
    Identical structure to real Keccak: A_i ^= (~A_{i+1}) & A_{i+2}
    Maps to Toffoli gates in a quantum circuit.
    """
    b = len(state)
    return [state[i] ^ ((not state[(i + 1) % b]) & state[(i + 2) % b]) for i in range(b)]


def iota(state: list, RC: list) -> list:
    """Round constant injection (ι): XOR state with round constant RC."""
    return [s ^ rc for s, rc in zip(state, RC)]


def keccak_f_toy(state: list, RC_list: list) -> list:
    """Apply N_ROUNDS of the toy permutation: ρ → χ → ι."""
    for RC in RC_list:
        state = rho(state)
        state = chi(state)
        state = iota(state, RC)
    return state


def keccak_toy(input_bits: list, n_rounds: int = N_ROUNDS, RC_list: list = None) -> list:
    """
    Toy Keccak sponge hash.

    Parameters
    ----------
    input_bits : list of 4 ints (0 or 1)
    n_rounds   : number of permutation rounds (default 2)
    RC_list    : list of round constants; uses [1,0,1,0,1,0,1,0] × n_rounds if None

    Returns
    -------
    list of 4 ints (0 or 1) — the hash output
    """
    state = [0] * B
    state = [s ^ i for s, i in zip(state, input_bits + [0] * (B - len(input_bits)))]
    if RC_list is None:
        RC_list = [[1, 0, 1, 0, 1, 0, 1, 0]] * n_rounds
    state = keccak_f_toy(state, RC_list)
    return state[:L]


def build_lookup() -> dict:
    """Return the full 4-bit input → 4-bit hash lookup table (16 entries)."""
    return {
        tuple([(i >> (3 - b)) & 1 for b in range(4)]): tuple(
            keccak_toy([(i >> (3 - b)) & 1 for b in range(4)])
        )
        for i in range(16)
    }
