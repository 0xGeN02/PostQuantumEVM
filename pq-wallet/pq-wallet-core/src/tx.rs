//! Post-quantum transaction types (local, minimal — no alloy-consensus dependency).
//!
//! Wire format (EIP-2718 type 0x50):
//! ```text
//! 0x50 || RLP([
//!   chain_id,
//!   nonce,
//!   gas_price,
//!   gas_limit,
//!   to,          -- 20 bytes or empty for contract creation
//!   value,
//!   input,
//!   signature,   -- raw bytes (3309)
//!   public_key,  -- raw bytes (1952)
//! ])
//! ```

use alloy_primitives::{Address, B256};
use alloy_rlp::{Encodable, RlpEncodable};
use serde::{Deserialize, Serialize};
use sha3::{Shake256, digest::{ExtendableOutput, Update, XofReader}};

/// EIP-2718 transaction type for PQ transactions.
///
/// `0x50` ('P') — avoids collision with EIP-7702 (type 4) and maps to
/// revm `TransactionType::Custom` so Prague-era validation is skipped.
pub const PQ_TX_TYPE: u8 = 0x50;

/// Unsigned post-quantum transaction fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PqTxRequest {
    pub chain_id: u64,
    pub nonce: u64,
    /// Recipient. `None` = contract creation.
    pub to: Option<Address>,
    /// Value in wei.
    pub value: u128,
    pub gas_limit: u64,
    /// Gas price in wei.
    pub gas_price: u128,
    /// Calldata / init code.
    pub input: Vec<u8>,
}

impl PqTxRequest {
    /// Canonical signing hash: `shake256(0x50 || chain_id || nonce || ..., 32)`.
    ///
    /// Uses SHAKE-256 (XOF) for quantum-safe hashing, aligned with ML-DSA-65.
    pub fn signing_hash(&self) -> B256 {
        let mut h = Shake256::default();
        h.update(&[PQ_TX_TYPE]);
        h.update(&self.chain_id.to_be_bytes());
        h.update(&self.nonce.to_be_bytes());
        h.update(&self.gas_price.to_be_bytes());
        h.update(&self.gas_limit.to_be_bytes());
        match &self.to {
            Some(addr) => {
                h.update(&[1u8]);
                h.update(addr.as_slice());
            }
            None => h.update(&[0u8]),
        }
        h.update(&self.value.to_be_bytes());
        h.update(&self.input);
        let mut hash = [0u8; 32];
        h.finalize_xof().read(&mut hash);
        B256::from(hash)
    }
}

// ─── RLP helper for encoding ─────────────────────────────────────────────────

/// RLP-encodable struct matching the node's expected wire format.
#[derive(RlpEncodable)]
struct PqTxRlpFields {
    chain_id: u64,
    nonce: u64,
    gas_price: u128,
    gas_limit: u64,
    to: alloy_rlp::Bytes,
    value: alloy_primitives::U256,
    input: alloy_rlp::Bytes,
    signature: alloy_rlp::Bytes,
    public_key: alloy_rlp::Bytes,
}

// ─── PqSignedTx ──────────────────────────────────────────────────────────────

/// A signed post-quantum transaction, ready to broadcast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PqSignedTx {
    pub tx: PqTxRequest,
    /// Raw ML-DSA-65 signature bytes (3309 bytes).
    pub sig_bytes: Vec<u8>,
    /// Raw ML-DSA-65 public key bytes (1952 bytes).
    pub pk_bytes: Vec<u8>,
    /// Transaction hash.
    pub hash: B256,
}

impl PqSignedTx {
    pub(crate) fn new(tx: PqTxRequest, sig_bytes: Vec<u8>, pk_bytes: Vec<u8>) -> Self {
        let hash = Self::compute_hash(&tx, &sig_bytes, &pk_bytes);
        Self { tx, sig_bytes, pk_bytes, hash }
    }

    /// Encode as EIP-2718 wire format:
    /// `0x50 || RLP([chain_id, nonce, gas_price, gas_limit, to, value, input, sig, pk])`
    ///
    /// This format is compatible with the node's `Decodable2718` implementation.
    pub fn encode(&self) -> Vec<u8> {
        let fields = PqTxRlpFields {
            chain_id: self.tx.chain_id,
            nonce: self.tx.nonce,
            gas_price: self.tx.gas_price,
            gas_limit: self.tx.gas_limit,
            to: match self.tx.to {
                Some(addr) => alloy_rlp::Bytes::copy_from_slice(addr.as_slice()),
                None => alloy_rlp::Bytes::new(),
            },
            value: alloy_primitives::U256::from(self.tx.value),
            input: alloy_rlp::Bytes::copy_from_slice(&self.tx.input),
            signature: alloy_rlp::Bytes::copy_from_slice(&self.sig_bytes),
            public_key: alloy_rlp::Bytes::copy_from_slice(&self.pk_bytes),
        };

        let mut out = Vec::with_capacity(1 + fields.length());
        out.push(PQ_TX_TYPE);
        fields.encode(&mut out);
        out
    }

    /// Hex-encoded signature (for display/JSON).
    pub fn signature_hex(&self) -> String {
        hex::encode(&self.sig_bytes)
    }

    /// Hex-encoded public key (for display/JSON).
    pub fn public_key_hex(&self) -> String {
        hex::encode(&self.pk_bytes)
    }

    fn compute_hash(tx: &PqTxRequest, sig: &[u8], pk: &[u8]) -> B256 {
        let mut h = Shake256::default();
        h.update(&[PQ_TX_TYPE]);
        h.update(tx.signing_hash().as_slice());
        h.update(sig);
        h.update(pk);
        let mut hash = [0u8; 32];
        h.finalize_xof().read(&mut hash);
        B256::from(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tx() -> PqTxRequest {
        PqTxRequest {
            chain_id: 20561,
            nonce: 0,
            to: Some(Address::from([0xab; 20])),
            value: 1_000_000_000_000_000_000, // 1 ETH
            gas_limit: 21_000,
            gas_price: 1_000_000_000,
            input: vec![],
        }
    }

    #[test]
    fn signing_hash_is_deterministic() {
        let tx = make_tx();
        assert_eq!(tx.signing_hash(), tx.signing_hash());
    }

    #[test]
    fn encode_starts_with_type_byte() {
        let tx = make_tx();
        let signed = PqSignedTx::new(tx, vec![0u8; 3309], vec![0u8; 1952]);
        let encoded = signed.encode();
        assert_eq!(encoded[0], PQ_TX_TYPE);
        // RLP list starts after type byte
        assert!(encoded.len() > 5300, "encoded tx should be >5KB (sig=3309 + pk=1952 + fields)");
    }

    #[test]
    fn encode_is_valid_rlp() {
        let tx = make_tx();
        let signed = PqSignedTx::new(tx, vec![0xaa; 3309], vec![0xbb; 1952]);
        let encoded = signed.encode();
        // Skip type byte, decode the RLP
        let mut buf = &encoded[1..];
        let decoded = alloy_rlp::Header::decode(&mut buf);
        assert!(decoded.is_ok(), "encoded body should be valid RLP");
        let header = decoded.unwrap();
        assert!(header.list, "should be an RLP list");
    }
}
