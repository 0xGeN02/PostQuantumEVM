"""
Grover's algorithm endpoints.

POST /grover/keccak   — run Grover search against the toy Keccak oracle
GET  /grover/lookup   — return the full 16-entry toy-Keccak lookup table
"""
from __future__ import annotations

import math
from fastapi import APIRouter, HTTPException

from lib.grover import grover_keccak
from lib.keccak_toy import build_lookup
from src.models import (
    GroverRequest,
    GroverResponse,
    KeccakLookupResponse,
)

router = APIRouter(prefix="/grover", tags=["Grover"])

# Build lookup once at import time (cheap, 16 entries)
_lookup = build_lookup()


@router.get(
    "/lookup",
    response_model=KeccakLookupResponse,
    summary="Toy-Keccak lookup table",
    description=(
        "Returns all 16 entries of the toy-Keccak (4-bit → 4-bit) hash function. "
        "Useful to pick a valid `target_hash` before calling `/grover/keccak`."
    ),
)
def get_lookup() -> KeccakLookupResponse:
    table = {
        "".join(map(str, inp)): "".join(map(str, out))
        for inp, out in _lookup.items()
    }
    return KeccakLookupResponse(lookup=table)


@router.post(
    "/keccak",
    response_model=GroverResponse,
    summary="Grover preimage attack on toy Keccak",
    description=(
        "Runs Grover's algorithm to find x such that `keccak_toy(x) == target_hash`. "
        "The toy hash operates on 4 bits (N=16 states). "
        "The optimal iteration count is ⌊π/4 · √(N/M)⌋ where M is the number of solutions."
    ),
)
def run_grover(body: GroverRequest) -> GroverResponse:
    target = tuple(body.target_hash)

    # Validate: all bits must be 0 or 1
    if any(b not in (0, 1) for b in target):
        raise HTTPException(
            status_code=422,
            detail="target_hash must contain only 0 and 1 values",
        )

    # Check the target actually exists in the lookup table
    solutions = [inp for inp, out in _lookup.items() if out == target]
    if not solutions:
        raise HTTPException(
            status_code=422,
            detail=f"No preimage exists for target_hash {list(target)}. "
                   f"Use GET /grover/lookup to see valid hashes.",
        )

    n = 4
    M = len(solutions)
    n_iter = max(1, int(math.floor(math.pi / 4 * math.sqrt(2**n / M))))

    _circuit, counts, solutions_found = grover_keccak(target, n=n, shots=body.shots)

    # counts keys are reversed-bit strings from Qiskit; normalise to left-to-right
    normalised_counts: dict[str, int] = {
        k[::-1]: v for k, v in counts.items()
    }

    top = max(normalised_counts, key=normalised_counts.get) if normalised_counts else None
    solutions_as_lists = [list(s) for s in solutions_found]
    top_as_bits = [int(b) for b in top] if top else []
    success = tuple(top_as_bits) in solutions_found if top else False

    return GroverResponse(
        target_hash=list(target),
        solutions=solutions_as_lists,
        iterations=n_iter,
        shots=body.shots,
        counts=normalised_counts,
        top_measurement=top,
        success=success,
    )
