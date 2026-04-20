//! Error types for pq-wallet-core.

use thiserror::Error;

/// All errors that can occur in the wallet.
#[derive(Debug, Error)]
pub enum WalletError {
    // ── Keystore ──────────────────────────────────────────────────────────────
    /// Failed to read or write the keystore file.
    #[error("keystore I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialisation/deserialisation failed.
    #[error("keystore JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Wrong passphrase or corrupted keystore.
    #[error("decryption failed — wrong passphrase or corrupted file")]
    DecryptionFailed,

    /// The encrypted payload has an invalid length.
    #[error("invalid keystore payload length")]
    InvalidPayload,

    // ── Crypto ────────────────────────────────────────────────────────────────
    /// The raw signing key bytes have an unexpected length.
    #[error("invalid signing key bytes")]
    InvalidSigningKey,

    /// The raw verifying key bytes have an unexpected length.
    #[error("invalid public key bytes: {0}")]
    InvalidPublicKey(String),

    /// Signature bytes are malformed.
    #[error("invalid signature bytes: {0}")]
    InvalidSignature(String),

    // ── RPC ───────────────────────────────────────────────────────────────────
    /// HTTP transport error.
    #[error("RPC transport error: {0}")]
    RpcTransport(#[from] reqwest::Error),

    /// The JSON-RPC response contained an error object.
    #[error("RPC error {code}: {message}")]
    RpcError { code: i64, message: String },

    /// The RPC response could not be parsed.
    #[error("RPC response parse error: {0}")]
    RpcParse(String),

    // ── Transaction ───────────────────────────────────────────────────────────
    /// Hex decoding of an address or hash failed.
    #[error("hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),
}
