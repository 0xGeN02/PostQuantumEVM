//! # kyber
//!
//! A wrapper around the [`ml-kem`](https://docs.rs/ml-kem) crate providing a
//! clean, ergonomic interface to CRYSTALS-Kyber (ML-KEM) as standardised in
//! [NIST FIPS 203](https://csrc.nist.gov/pubs/fips/203/final).
//!
//! ## Security levels
//!
//! | Variant       | NIST category | Comparable classical security |
//! |---------------|---------------|-------------------------------|
//! | `MlKem512`    | 1             | AES-128                       |
//! | `MlKem768`    | 3             | AES-192 (**recommended**)     |
//! | `MlKem1024`   | 5             | AES-256                       |
//!
//! ## Quick-start
//!
//! ```rust
//! use kyber::kyber768;
//!
//! let (dk, ek) = kyber768::keygen();
//! let (ct, shared_send) = kyber768::encapsulate(&ek);
//! let shared_recv = kyber768::decapsulate(&dk, &ct);
//! assert_eq!(shared_send.as_slice(), shared_recv.as_slice());
//! ```

pub use ml_kem::{MlKem1024, MlKem512, MlKem768};

// ─── ML-KEM-512 ──────────────────────────────────────────────────────────────

/// ML-KEM-512 (NIST security category 1).
pub mod kyber512 {
    use ml_kem::{
        kem::{Decapsulate, Encapsulate, Kem},
        Ciphertext, DecapsulationKey, EncapsulationKey, MlKem512, SharedKey,
    };

    /// Generate a fresh ML-KEM-512 keypair.
    pub fn keygen() -> (DecapsulationKey<MlKem512>, EncapsulationKey<MlKem512>) {
        MlKem512::generate_keypair()
    }

    /// Encapsulate a shared secret to the holder of `ek`.
    /// Returns `(ciphertext, shared_key)`.
    pub fn encapsulate(ek: &EncapsulationKey<MlKem512>) -> (Ciphertext<MlKem512>, SharedKey) {
        ek.encapsulate()
    }

    /// Recover the shared key from `ct` using the decapsulation key `dk`.
    pub fn decapsulate(dk: &DecapsulationKey<MlKem512>, ct: &Ciphertext<MlKem512>) -> SharedKey {
        dk.decapsulate(ct)
    }
}

// ─── ML-KEM-768 ──────────────────────────────────────────────────────────────

/// ML-KEM-768 (NIST security category 3) — **recommended for most uses**.
pub mod kyber768 {
    use ml_kem::{
        kem::{Decapsulate, Encapsulate, Kem},
        Ciphertext, DecapsulationKey, EncapsulationKey, MlKem768, SharedKey,
    };

    /// Generate a fresh ML-KEM-768 keypair.
    pub fn keygen() -> (DecapsulationKey<MlKem768>, EncapsulationKey<MlKem768>) {
        MlKem768::generate_keypair()
    }

    /// Encapsulate a shared secret to the holder of `ek`.
    pub fn encapsulate(ek: &EncapsulationKey<MlKem768>) -> (Ciphertext<MlKem768>, SharedKey) {
        ek.encapsulate()
    }

    /// Recover the shared key from `ct` using the decapsulation key `dk`.
    pub fn decapsulate(dk: &DecapsulationKey<MlKem768>, ct: &Ciphertext<MlKem768>) -> SharedKey {
        dk.decapsulate(ct)
    }
}

// ─── ML-KEM-1024 ─────────────────────────────────────────────────────────────

/// ML-KEM-1024 (NIST security category 5).
pub mod kyber1024 {
    use ml_kem::{
        kem::{Decapsulate, Encapsulate, Kem},
        Ciphertext, DecapsulationKey, EncapsulationKey, MlKem1024, SharedKey,
    };

    /// Generate a fresh ML-KEM-1024 keypair.
    pub fn keygen() -> (DecapsulationKey<MlKem1024>, EncapsulationKey<MlKem1024>) {
        MlKem1024::generate_keypair()
    }

    /// Encapsulate a shared secret to the holder of `ek`.
    pub fn encapsulate(ek: &EncapsulationKey<MlKem1024>) -> (Ciphertext<MlKem1024>, SharedKey) {
        ek.encapsulate()
    }

    /// Recover the shared key from `ct` using the decapsulation key `dk`.
    pub fn decapsulate(dk: &DecapsulationKey<MlKem1024>, ct: &Ciphertext<MlKem1024>) -> SharedKey {
        dk.decapsulate(ct)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! kem_roundtrip {
        ($name:ident, $mod:ident) => {
            #[test]
            fn $name() {
                let (dk, ek) = $mod::keygen();
                let (ct, k_send) = $mod::encapsulate(&ek);
                let k_recv = $mod::decapsulate(&dk, &ct);
                assert_eq!(
                    k_send.as_slice(),
                    k_recv.as_slice(),
                    "shared keys must match after roundtrip"
                );
            }
        };
    }

    kem_roundtrip!(roundtrip_kyber512, kyber512);
    kem_roundtrip!(roundtrip_kyber768, kyber768);
    kem_roundtrip!(roundtrip_kyber1024, kyber1024);

    #[test]
    fn wrong_ciphertext_gives_different_key() {
        let (dk, ek) = kyber768::keygen();
        let (_, k_real) = kyber768::encapsulate(&ek);

        let (ct_other, _) = kyber768::encapsulate(&ek);
        let k_other = kyber768::decapsulate(&dk, &ct_other);

        assert_ne!(
            k_real.as_slice(),
            k_other.as_slice(),
            "different ciphertexts should yield different shared keys"
        );
    }
}
