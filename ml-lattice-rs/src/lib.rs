//! # ml-lattice-rs
//!
//! Rust implementations of NIST-standardised post-quantum lattice-based
//! cryptographic algorithms for use in the Post-Quantum EVM project.
//!
//! | Crate        | Algorithm      | Standard         | Purpose               |
//! |--------------|----------------|------------------|-----------------------|
//! | [`kyber`]    | ML-KEM (Kyber) | NIST FIPS 203    | Key encapsulation     |
//! | [`dilithium`]| ML-DSA         | NIST FIPS 204    | Digital signatures    |
//!
//! ## Example
//!
//! ```rust
//! use ml_lattice_rs::{kyber, dilithium};
//! use ml_lattice_rs::dilithium::signature::Keypair;
//!
//! // ── Key Encapsulation (Kyber / ML-KEM) ───────────────────────────────────
//! let (dk, ek) = kyber::kyber768::keygen();
//! let (ct, shared_send) = kyber::kyber768::encapsulate(&ek);
//! let shared_recv = kyber::kyber768::decapsulate(&dk, &ct);
//! assert_eq!(shared_send.as_slice(), shared_recv.as_slice());
//!
//! // ── Digital Signatures (Dilithium / ML-DSA) ──────────────────────────────
//! let sk = dilithium::dilithium65::keygen();
//! let vk = sk.verifying_key();
//! let msg = b"post-quantum transaction";
//! let sig = dilithium::dilithium65::sign(&sk, msg);
//! dilithium::dilithium65::verify(&vk, msg, &sig).unwrap();
//! ```

pub use dilithium;
pub use kyber;
