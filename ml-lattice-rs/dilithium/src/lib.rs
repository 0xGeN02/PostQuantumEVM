//! # dilithium
//!
//! A wrapper around the [`ml-dsa`](https://docs.rs/ml-dsa) crate providing a
//! clean, ergonomic interface to CRYSTALS-Dilithium (ML-DSA) as standardised in
//! [NIST FIPS 204](https://csrc.nist.gov/pubs/fips/204/final).
//!
//! ## Security levels
//!
//! | Variant    | NIST cat | Signing key | Verifying key | Signature  |
//! |------------|----------|-------------|---------------|------------|
//! | `MlDsa44`  | 2        | 2528 B      | 1312 B        | 2420 B     |
//! | `MlDsa65`  | 3        | 4032 B      | 1952 B        | 3309 B (**recommended**) |
//! | `MlDsa87`  | 5        | 4896 B      | 2592 B        | 4627 B     |
//!
//! ## Quick-start
//!
//! ```rust
//! use dilithium::dilithium65;
//! use dilithium::signature::Keypair;
//!
//! let sk = dilithium65::keygen();
//! let msg = b"post-quantum blockchain transaction";
//! let sig = dilithium65::sign(&sk, msg);
//! assert!(dilithium65::verify(&sk.verifying_key(), msg, &sig).is_ok());
//! ```

// Re-export common types so callers can use them without reaching into ml_dsa.
pub use ml_dsa::{
    EncodedSignature, EncodedVerifyingKey, KeyGen, MlDsa44, MlDsa65, MlDsa87, Signature,
    SigningKey, VerifyingKey,
};
// Re-export the signature crate so callers can access `Keypair`, `Signer`, etc.
pub use ml_dsa::signature;

use thiserror::Error;

// ─── Error type ──────────────────────────────────────────────────────────────

/// Errors returned by dilithium operations.
#[derive(Debug, Error)]
pub enum DilithiumError {
    /// Signature verification failed.
    #[error("signature verification failed")]
    InvalidSignature,
}

// ─── ML-DSA-44 ───────────────────────────────────────────────────────────────

/// ML-DSA-44 (NIST security category 2).
pub mod dilithium44 {
    use getrandom::{rand_core::UnwrapErr, SysRng};
    use ml_dsa::{
        signature::{Signer, Verifier},
        KeyGen, MlDsa44, Signature, SigningKey, VerifyingKey,
    };

    use super::DilithiumError;

    /// Generate a fresh ML-DSA-44 signing key.
    pub fn keygen() -> SigningKey<MlDsa44> {
        MlDsa44::key_gen(&mut UnwrapErr(SysRng))
    }

    /// Sign `msg` deterministically with `sk`.
    pub fn sign(sk: &SigningKey<MlDsa44>, msg: &[u8]) -> Signature<MlDsa44> {
        sk.sign(msg)
    }

    /// Verify `sig` over `msg` using the verifying key `vk`.
    pub fn verify(
        vk: &VerifyingKey<MlDsa44>,
        msg: &[u8],
        sig: &Signature<MlDsa44>,
    ) -> Result<(), DilithiumError> {
        vk.verify(msg, sig)
            .map_err(|_| DilithiumError::InvalidSignature)
    }
}

// ─── ML-DSA-65 ───────────────────────────────────────────────────────────────

/// ML-DSA-65 (NIST security category 3) — **recommended for most uses**.
pub mod dilithium65 {
    use getrandom::{rand_core::UnwrapErr, SysRng};
    use ml_dsa::{
        signature::{Signer, Verifier},
        KeyGen, MlDsa65, Signature, SigningKey, VerifyingKey,
    };

    use super::DilithiumError;

    /// Generate a fresh ML-DSA-65 signing key.
    pub fn keygen() -> SigningKey<MlDsa65> {
        MlDsa65::key_gen(&mut UnwrapErr(SysRng))
    }

    /// Sign `msg` deterministically with `sk`.
    pub fn sign(sk: &SigningKey<MlDsa65>, msg: &[u8]) -> Signature<MlDsa65> {
        sk.sign(msg)
    }

    /// Verify `sig` over `msg` using the verifying key `vk`.
    pub fn verify(
        vk: &VerifyingKey<MlDsa65>,
        msg: &[u8],
        sig: &Signature<MlDsa65>,
    ) -> Result<(), DilithiumError> {
        vk.verify(msg, sig)
            .map_err(|_| DilithiumError::InvalidSignature)
    }
}

// ─── ML-DSA-87 ───────────────────────────────────────────────────────────────

/// ML-DSA-87 (NIST security category 5).
pub mod dilithium87 {
    use getrandom::{rand_core::UnwrapErr, SysRng};
    use ml_dsa::{
        signature::{Signer, Verifier},
        KeyGen, MlDsa87, Signature, SigningKey, VerifyingKey,
    };

    use super::DilithiumError;

    /// Generate a fresh ML-DSA-87 signing key.
    pub fn keygen() -> SigningKey<MlDsa87> {
        MlDsa87::key_gen(&mut UnwrapErr(SysRng))
    }

    /// Sign `msg` deterministically with `sk`.
    pub fn sign(sk: &SigningKey<MlDsa87>, msg: &[u8]) -> Signature<MlDsa87> {
        sk.sign(msg)
    }

    /// Verify `sig` over `msg` using the verifying key `vk`.
    pub fn verify(
        vk: &VerifyingKey<MlDsa87>,
        msg: &[u8],
        sig: &Signature<MlDsa87>,
    ) -> Result<(), DilithiumError> {
        vk.verify(msg, sig)
            .map_err(|_| DilithiumError::InvalidSignature)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use ml_dsa::signature::Keypair;

    use super::*;

    const MSG: &[u8] = b"Hello, Post-Quantum World!";

    macro_rules! dsa_roundtrip {
        ($name:ident, $mod:ident) => {
            #[test]
            fn $name() {
                let sk = $mod::keygen();
                let vk = sk.verifying_key();
                let sig = $mod::sign(&sk, MSG);
                $mod::verify(&vk, MSG, &sig).expect("valid signature must verify");
            }
        };
    }

    dsa_roundtrip!(roundtrip_dilithium44, dilithium44);
    dsa_roundtrip!(roundtrip_dilithium65, dilithium65);
    dsa_roundtrip!(roundtrip_dilithium87, dilithium87);

    #[test]
    fn tampered_message_fails_verification() {
        let sk = dilithium65::keygen();
        let vk = sk.verifying_key();
        let sig = dilithium65::sign(&sk, MSG);

        let tampered = b"Hello, Classical World!";
        assert!(
            dilithium65::verify(&vk, tampered, &sig).is_err(),
            "verification must fail on tampered message"
        );
    }

    #[test]
    fn wrong_key_fails_verification() {
        let sk1 = dilithium65::keygen();
        let sk2 = dilithium65::keygen();
        let sig = dilithium65::sign(&sk1, MSG);

        assert!(
            dilithium65::verify(&sk2.verifying_key(), MSG, &sig).is_err(),
            "verification must fail with wrong key"
        );
    }
}
