//! Post-quantum transaction types (local, minimal — no alloy-consensus dependency).

use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

/// EIP-2718 transaction type for PQ transactions.
pub const PQ_TX_TYPE: u8 = 0x04;

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
    /// Canonical signing hash: `keccak256(0x04 || chain_id || nonce || ...)`.
    pub fn signing_hash(&self) -> B256 {
        let mut h = Keccak256::new();
        h.update([PQ_TX_TYPE]);
        h.update(self.chain_id.to_be_bytes());
        h.update(self.nonce.to_be_bytes());
        h.update(self.gas_price.to_be_bytes());
        h.update(self.gas_limit.to_be_bytes());
        match &self.to {
            Some(addr) => {
                h.update([1u8]);
                h.update(addr.as_slice());
            }
            None => h.update([0u8]),
        }
        h.update(self.value.to_be_bytes());
        h.update(&self.input);
        B256::from_slice(&h.finalize())
    }
}

/// A signed post-quantum transaction, ready to broadcast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PqSignedTx {
    pub tx: PqTxRequest,
    /// Hex-encoded ML-DSA-65 signature (3309 bytes → 6618 hex chars).
    pub signature: String,
    /// Hex-encoded ML-DSA-65 public key (1952 bytes → 3904 hex chars).
    pub public_key: String,
    /// Transaction hash.
    pub hash: B256,
}

impl PqSignedTx {
    pub(crate) fn new(tx: PqTxRequest, sig_bytes: Vec<u8>, pk_bytes: Vec<u8>) -> Self {
        let hash = Self::compute_hash(&tx, &sig_bytes, &pk_bytes);
        Self {
            tx,
            signature: hex::encode(&sig_bytes),
            public_key: hex::encode(&pk_bytes),
            hash,
        }
    }

    /// Encode as the raw bytes that would go on the wire (simplified).
    ///
    /// Format: `0x04 || signing_hash(32) || sig_bytes || pk_bytes`
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(PQ_TX_TYPE);
        out.extend_from_slice(self.tx.signing_hash().as_slice());
        out.extend_from_slice(&hex::decode(&self.signature).unwrap_or_default());
        out.extend_from_slice(&hex::decode(&self.public_key).unwrap_or_default());
        out
    }

    fn compute_hash(tx: &PqTxRequest, sig: &[u8], pk: &[u8]) -> B256 {
        let mut h = Keccak256::new();
        h.update([PQ_TX_TYPE]);
        h.update(tx.signing_hash().as_slice());
        h.update(sig);
        h.update(pk);
        B256::from_slice(&h.finalize())
    }
}
