"""
Shor's algorithm endpoints.

POST /shor/ecdlp   — recover private key k from public key Q using Shor's ECDLP
GET  /shor/curve   — toy secp256k1 curve parameters and subgroup points
"""
from __future__ import annotations

from fastapi import APIRouter, HTTPException

from lib.secp256k1_toy import (
    INFINITY,
    G,
    N,
    P,
    A,
    B,
    generate_keypair,
    subgroup_points,
)
from lib.shor import ORDER, shor_ecdlp
from src.models import CurveInfoResponse, ShorRequest, ShorResponse

router = APIRouter(prefix="/shor", tags=["Shor"])


@router.get(
    "/curve",
    response_model=CurveInfoResponse,
    summary="Toy secp256k1 curve parameters",
    description=(
        "Returns the parameters of the toy secp256k1 curve used in the Shor simulation: "
        "y² = x³ + 7 (mod 17). Includes all points in the cyclic subgroup ⟨G⟩."
    ),
)
def get_curve_info() -> CurveInfoResponse:
    points = subgroup_points()
    serialised = [None if pt is INFINITY else list(pt) for pt in points]
    return CurveInfoResponse(
        p=P,
        a=A,
        b=B,
        G=list(G),
        N=N,
        order=ORDER,
        subgroup_points=serialised,
    )


@router.post(
    "/ecdlp",
    response_model=ShorResponse,
    summary="Shor ECDLP attack on toy secp256k1",
    description=(
        "Recovers the private key k from a public key Q = k·G on the toy curve "
        "(p=17) using the quantum Shor ECDLP algorithm. "
        "Omit `public_key` to let the server generate a fresh random keypair."
    ),
)
def run_shor(body: ShorRequest) -> ShorResponse:
    if body.public_key is not None:
        x, y = body.public_key
        Q: tuple = (x, y)

        # Validate Q is a real point on the toy curve
        from lib.secp256k1_toy import is_on_curve
        if not is_on_curve(Q):
            raise HTTPException(
                status_code=422,
                detail=f"Point {Q} is not on the toy secp256k1 curve (p={P}, b={B}). "
                       f"Use GET /shor/curve to see valid subgroup points.",
            )

        # Derive the true private key index from the precomputed subgroup table
        from lib.shor import _group_points, _point_to_idx
        if Q not in _group_points:
            raise HTTPException(
                status_code=422,
                detail=f"Point {Q} is on the curve but not in the subgroup ⟨G⟩. "
                       f"Use GET /shor/curve to see valid points.",
            )
        k_true = _point_to_idx(Q)
    else:
        # Generate a fresh random keypair
        k_true, Q = generate_keypair()
        # generate_keypair may return INFINITY for k=0 (extremely unlikely but guard)
        if Q is INFINITY:
            k_true, Q = generate_keypair()

    k_recovered, counts, _probs = shor_ecdlp(Q, shots=body.shots)

    # Serialise top-10 measurement pairs for the response
    top_counts = dict(
        sorted(
            {f"(s={s},t={t})": c for (s, t), c in counts.items()}.items(),
            key=lambda item: -item[1],
        )[:10]
    )

    return ShorResponse(
        public_key=list(Q),
        private_key_true=k_true,
        private_key_recovered=k_recovered,
        success=(k_recovered is not None and k_recovered == k_true),
        shots=body.shots,
        top_counts=top_counts,
    )
