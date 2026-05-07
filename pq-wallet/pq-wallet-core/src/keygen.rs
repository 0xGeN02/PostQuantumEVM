//! ML-DSA-65 keypair — generation and address derivation.

use alloy_primitives::Address;
use dilithium::{signature::Keypair, MlDsa65, SigningKey, VerifyingKey};
use sha3::{Shake256, digest::{ExtendableOutput, Update, XofReader}};

use crate::error::WalletError;

/// An ML-DSA-65 keypair with its derived Ethereum address.
#[derive(Debug)]
pub struct PqKeypair {
    /// The ML-DSA-65 signing key (contains the verifying key).
    pub(crate) sk: SigningKey<MlDsa65>,
}

impl PqKeypair {
    /// Generate a fresh ML-DSA-65 keypair using the OS RNG.
    pub fn generate() -> Self {
        Self {
            sk: dilithium::dilithium65::keygen(),
        }
    }

    /// Rebuild a keypair from a 32-byte seed.
    ///
    /// The seed is the value returned by [`PqKeypair::seed_bytes`].
    pub fn from_seed_bytes(bytes: &[u8]) -> Result<Self, WalletError> {
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| WalletError::InvalidSigningKey)?;
        use dilithium::KeyGen;
        use hybrid_array::Array;
        use typenum::U32;
        let seed: Array<u8, U32> = arr.into();
        Ok(Self {
            sk: MlDsa65::from_seed(&seed),
        })
    }

    /// The raw 32-byte seed used to derive this keypair.
    ///
    /// **NEVER share this — it is your private key.**
    pub fn seed_bytes(&self) -> [u8; 32] {
        self.sk.to_seed().into()
    }

    /// The ML-DSA-65 verifying (public) key.
    pub fn verifying_key(&self) -> VerifyingKey<MlDsa65> {
        self.sk.verifying_key()
    }

    /// Raw encoded verifying key bytes (1952 bytes for ML-DSA-65).
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.sk.verifying_key().encode().as_slice().to_vec()
    }

    /// Derive the Ethereum address: `shake256(pk_bytes, 32)[12..]`.
    ///
    /// Uses SHAKE-256 (XOF) aligned with ML-DSA-65 which uses SHAKE-256 internally.
    pub fn address(&self) -> Address {
        let pk_bytes = self.public_key_bytes();
        let mut hasher = Shake256::default();
        hasher.update(&pk_bytes);
        let mut hash = [0u8; 32];
        hasher.finalize_xof().read(&mut hash);
        Address::from_slice(&hash[12..])
    }

    /// Save this keypair to an encrypted keystore file.
    pub fn save(&self, path: &std::path::Path, passphrase: &str) -> Result<(), WalletError> {
        crate::keystore::Keystore::save(self, path, passphrase)
    }

    /// Sign an arbitrary message with ML-DSA-65. Returns the raw signature bytes.
    pub fn sign_message(&self, msg: &[u8]) -> Vec<u8> {
        use dilithium::signature::Signer;
        self.sk.sign(msg).encode().as_slice().to_vec()
    }
}
