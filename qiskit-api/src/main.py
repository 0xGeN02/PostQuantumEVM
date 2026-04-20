"""
Qiskit API — Quantum Attack Simulation Server
=============================================

Phase 3 of the Post-Quantum EVM project.

Exposes REST endpoints to simulate quantum attacks against classical
cryptographic primitives used in Ethereum:

  Grover's algorithm  →  hash preimage search (toy Keccak)
  Shor's algorithm    →  ECDLP on toy secp256k1 (private-key recovery)

Run locally:
    uvicorn src.main:app --reload --port 8888

Docker:
    docker compose up
"""
from fastapi import FastAPI
from fastapi.responses import JSONResponse

from src.routers import grover, shor

app = FastAPI(
    title="Qiskit API",
    version="0.2.0",
    description=(
        "Quantum attack simulation API for the Post-Quantum EVM project. "
        "Implements Grover's algorithm (hash preimage search) and "
        "Shor's algorithm (ECDLP private-key recovery) using Qiskit + Aer."
    ),
    contact={
        "name": "0xGeN02",
        "url": "https://github.com/0xGeN02/PostQuantumEVM",
    },
    license_info={"name": "Apache 2.0"},
)

# ── Routers ──────────────────────────────────────────────────────────────────
app.include_router(grover.router)
app.include_router(shor.router)


# ── Root health-check ────────────────────────────────────────────────────────
@app.get("/", tags=["Health"], summary="Health check")
def health_check():
    return {"status": "ok", "service": "qiskit-api", "version": "0.2.0"}
