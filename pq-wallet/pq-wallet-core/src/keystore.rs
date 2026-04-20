//! Encrypted keystore — persists an ML-DSA-65 signing key on disk.
//!
//! # Format
//!
//! The keystore is a JSON file with this structure:
//! ```json
//! {
//!   "version": 1,
//!   "address": "0xabc...",
//!   "public_key": "<hex>",
//!   "crypto": {
//!     "kdf": "argon2id",
//!     "kdf_params": { "m_cost": 65536, "t_cost": 3, "p_cost": 4, "salt": "<hex>" },
//!     "cipher": "aes-256-gcm",
//!     "cipher_params": { "iv": "<hex>" },
//!     "ciphertext": "<hex>"
//!   }
//! }
//! ```
//!
//! # Security
//!
//! - Key derivation: Argon2id (m=64MB, t=3, p=4)
//! - Encryption: AES-256-GCM (authenticated)
//! - The 32-byte seed is encrypted (not the full expanded key)

use std::path::Path;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use argon2::{Argon2, Params, Version};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::{error::WalletError, keygen::PqKeypair};

// ─── Keystore JSON schema ────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Keystore {
    /// Format version.
    pub version: u32,
    /// Derived Ethereum address (informational).
    pub address: String,
    /// Hex-encoded ML-DSA-65 public key (1952 bytes).
    pub public_key: String,
    /// Encrypted key material.
    pub crypto: KeystoreCrypto,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeystoreCrypto {
    pub kdf: String,
    pub kdf_params: KdfParams,
    pub cipher: String,
    pub cipher_params: CipherParams,
    /// Hex-encoded AES-256-GCM ciphertext (includes 16-byte auth tag).
    pub ciphertext: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KdfParams {
    /// Memory cost in KiB (64 MB).
    pub m_cost: u32,
    /// Iteration count.
    pub t_cost: u32,
    /// Parallelism.
    pub p_cost: u32,
    /// Hex-encoded 16-byte salt.
    pub salt: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CipherParams {
    /// Hex-encoded 12-byte IV/nonce.
    pub iv: String,
}

// ─── Constants ───────────────────────────────────────────────────────────────

/// Argon2id memory cost: 64 MB.
const ARGON2_M_COST: u32 = 64 * 1024;
/// Argon2id iterations.
const ARGON2_T_COST: u32 = 3;
/// Argon2id parallelism.
const ARGON2_P_COST: u32 = 4;

// ─── impl Keystore ───────────────────────────────────────────────────────────

impl Keystore {
    /// Encrypt `keypair` with `passphrase` and write to `path`.
    pub fn save(keypair: &PqKeypair, path: &Path, passphrase: &str) -> Result<(), WalletError> {
        let mut rng = rand::thread_rng();

        // Random salt (16 bytes) and IV (12 bytes)
        let mut salt = [0u8; 16];
        let mut iv = [0u8; 12];
        rng.fill_bytes(&mut salt);
        rng.fill_bytes(&mut iv);

        // Derive 32-byte AES key via Argon2id
        let aes_key = derive_key(passphrase, &salt)?;

        // Encrypt the 32-byte seed
        let seed = keypair.seed_bytes();
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&aes_key));
        let nonce = Nonce::from_slice(&iv);
        let ciphertext = cipher
            .encrypt(nonce, seed.as_ref())
            .map_err(|_| WalletError::DecryptionFailed)?;

        let ks = Keystore {
            version: 1,
            address: format!("{}", keypair.address()),
            public_key: hex::encode(keypair.public_key_bytes()),
            crypto: KeystoreCrypto {
                kdf: "argon2id".into(),
                kdf_params: KdfParams {
                    m_cost: ARGON2_M_COST,
                    t_cost: ARGON2_T_COST,
                    p_cost: ARGON2_P_COST,
                    salt: hex::encode(salt),
                },
                cipher: "aes-256-gcm".into(),
                cipher_params: CipherParams {
                    iv: hex::encode(iv),
                },
                ciphertext: hex::encode(ciphertext),
            },
        };

        let json = serde_json::to_string_pretty(&ks)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load and decrypt a keystore file with `passphrase`.
    ///
    /// Returns a [`PqKeypair`] on success, or [`WalletError::DecryptionFailed`]
    /// if the passphrase is wrong.
    pub fn load(path: &Path, passphrase: &str) -> Result<PqKeypair, WalletError> {
        let json = std::fs::read_to_string(path)?;
        let ks: Keystore = serde_json::from_str(&json)?;

        let salt = hex::decode(&ks.crypto.kdf_params.salt)?;
        let iv = hex::decode(&ks.crypto.cipher_params.iv)?;
        let ciphertext = hex::decode(&ks.crypto.ciphertext)?;

        let aes_key = derive_key(passphrase, &salt)?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&aes_key));
        let nonce = Nonce::from_slice(&iv);

        let seed_bytes = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| WalletError::DecryptionFailed)?;

        PqKeypair::from_seed_bytes(&seed_bytes)
    }

    /// Return just the address from a keystore file without decrypting.
    pub fn address_from_file(path: &Path) -> Result<String, WalletError> {
        let json = std::fs::read_to_string(path)?;
        let ks: Keystore = serde_json::from_str(&json)?;
        Ok(ks.address)
    }

    /// Return just the public key hex from a keystore file without decrypting.
    pub fn public_key_from_file(path: &Path) -> Result<String, WalletError> {
        let json = std::fs::read_to_string(path)?;
        let ks: Keystore = serde_json::from_str(&json)?;
        Ok(ks.public_key)
    }
}

// ─── Internal helpers ────────────────────────────────────────────────────────

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], WalletError> {
    let params = Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, Some(32))
        .map_err(|e| WalletError::RpcParse(e.to_string()))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| WalletError::RpcParse(e.to_string()))?;
    Ok(key)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keygen::PqKeypair;
    use tempfile::NamedTempFile;

    #[test]
    fn roundtrip_save_load() {
        let keypair = PqKeypair::generate();
        let address = keypair.address();
        let pk_bytes = keypair.public_key_bytes();

        let file = NamedTempFile::new().unwrap();
        Keystore::save(&keypair, file.path(), "correct-horse-battery-staple").unwrap();

        let loaded = Keystore::load(file.path(), "correct-horse-battery-staple").unwrap();
        assert_eq!(
            loaded.address(),
            address,
            "address must match after roundtrip"
        );
        assert_eq!(
            loaded.public_key_bytes(),
            pk_bytes,
            "public key must match after roundtrip"
        );
    }

    #[test]
    fn wrong_passphrase_fails() {
        let keypair = PqKeypair::generate();
        let file = NamedTempFile::new().unwrap();
        Keystore::save(&keypair, file.path(), "correct-passphrase").unwrap();

        let result = Keystore::load(file.path(), "wrong-passphrase");
        assert!(result.is_err(), "wrong passphrase must return an error");
        assert!(matches!(result.unwrap_err(), WalletError::DecryptionFailed));
    }

    #[test]
    fn address_from_file_does_not_need_passphrase() {
        let keypair = PqKeypair::generate();
        let expected = format!("{}", keypair.address());
        let file = NamedTempFile::new().unwrap();
        Keystore::save(&keypair, file.path(), "pass").unwrap();

        let addr = Keystore::address_from_file(file.path()).unwrap();
        assert_eq!(addr, expected);
    }
}
