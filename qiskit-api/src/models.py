"""
Shared Pydantic models for request/response schemas.
"""
from __future__ import annotations

from typing import Optional
from pydantic import BaseModel, Field


# ─── Grover ──────────────────────────────────────────────────────────────────

class GroverRequest(BaseModel):
    """
    Run Grover's algorithm searching for the preimage of a 4-bit toy-Keccak hash.

    `target_hash` must be a list of exactly 4 bits (0 or 1).
    """
    target_hash: list[int] = Field(
        ...,
        min_length=4,
        max_length=4,
        examples=[[1, 0, 1, 0]],
        description="4-bit target hash as a list of 0/1 values",
    )
    shots: int = Field(
        default=1024,
        ge=1,
        le=65536,
        description="Number of measurement shots for the quantum simulation",
    )


class GroverResponse(BaseModel):
    target_hash: list[int]
    solutions: list[list[int]] = Field(
        description="All preimages x such that keccak_toy(x) == target_hash"
    )
    iterations: int = Field(description="Optimal Grover iteration count used")
    shots: int
    counts: dict[str, int] = Field(
        description="Measurement histogram: bitstring → count"
    )
    top_measurement: Optional[str] = Field(
        description="Most-frequent measured bitstring"
    )
    success: bool = Field(
        description="True if top_measurement matches a known solution"
    )


# ─── Shor ────────────────────────────────────────────────────────────────────

class ShorRequest(BaseModel):
    """
    Run Shor's ECDLP algorithm to recover the private key k from a public key
    Q = (x, y) on the toy secp256k1 curve (p=17).

    Omit `public_key` to let the server generate a fresh random keypair.
    """
    public_key: Optional[list[int]] = Field(
        default=None,
        min_length=2,
        max_length=2,
        examples=[[6, 6]],
        description="Public key Q = [x, y] on the toy curve.  "
                    "Leave null to generate a random keypair.",
    )
    shots: int = Field(
        default=4096,
        ge=1,
        le=131072,
        description="Number of measurement samples",
    )


class ShorResponse(BaseModel):
    public_key: list[int] = Field(description="Public key Q = [x, y] used")
    private_key_true: int = Field(description="Ground-truth private key k")
    private_key_recovered: Optional[int] = Field(
        description="Private key recovered by Shor's algorithm (None if failed)"
    )
    success: bool = Field(description="True if recovered key matches ground truth")
    shots: int
    top_counts: dict[str, int] = Field(
        description="Top-10 (s, t) measurement pairs by frequency"
    )


# ─── Lookup / curve info ─────────────────────────────────────────────────────

class KeccakLookupResponse(BaseModel):
    lookup: dict[str, str] = Field(
        description="Full 16-entry lookup table: '0101' → '1100'"
    )


class CurveInfoResponse(BaseModel):
    p: int = Field(description="Field prime")
    a: int = Field(description="Curve coefficient a")
    b: int = Field(description="Curve coefficient b")
    G: list[int] = Field(description="Generator point [x, y]")
    N: int = Field(description="Order of G")
    order: int = Field(description="True cyclic order of G in its subgroup")
    subgroup_points: list[Optional[list[int]]] = Field(
        description="All points in <G>, None represents the point at infinity"
    )
