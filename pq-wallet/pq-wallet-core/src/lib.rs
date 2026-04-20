//! # pq-wallet-core
//!
//! Core library for the post-quantum wallet.
//!
//! Provides:
//! - [`keygen`] — ML-DSA-65 keypair generation
//! - [`keystore`] — encrypted key storage on disk (AES-256-GCM + Argon2id)
//! - [`signer`] — transaction signing
//! - [`rpc`] — minimal JSON-RPC client (balance, nonce, send)
//!
//! # Example
//!
//! ```no_run
//! use pq_wallet_core::keygen::PqKeypair;
//! use pq_wallet_core::keystore::Keystore;
//!
//! // Generate a new keypair
//! let keypair = PqKeypair::generate();
//! println!("Address: {}", keypair.address());
//!
//! // Save to encrypted keystore
//! let path = std::path::Path::new("/tmp/my-key.json");
//! keypair.save(path, "my-passphrase").unwrap();
//!
//! // Load it back
//! let loaded = Keystore::load(path, "my-passphrase").unwrap();
//! assert_eq!(keypair.address(), loaded.address());
//! ```

pub mod error;
pub mod keygen;
pub mod keystore;
pub mod rpc;
pub mod signer;
pub mod tx;

pub use error::WalletError;
pub use keygen::PqKeypair;
pub use keystore::Keystore;
pub use rpc::RpcClient;
pub use signer::PqSigner;
pub use tx::{PqTxRequest, PQ_TX_TYPE};
