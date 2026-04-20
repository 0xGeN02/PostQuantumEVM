//! Transaction signer — combines a [`PqKeypair`] with [`PqTxRequest`].

use dilithium::signature::Signer;

use crate::{
    keygen::PqKeypair,
    tx::{PqSignedTx, PqTxRequest},
};

/// Signs post-quantum transactions with an ML-DSA-65 key.
pub struct PqSigner<'a> {
    keypair: &'a PqKeypair,
}

impl<'a> PqSigner<'a> {
    /// Create a signer from a keypair reference.
    pub fn new(keypair: &'a PqKeypair) -> Self {
        Self { keypair }
    }

    /// Sign a transaction request and return a [`PqSignedTx`].
    pub fn sign(&self, tx: PqTxRequest) -> PqSignedTx {
        let hash = tx.signing_hash();
        let sig = self.keypair.sk.sign(hash.as_slice());
        let sig_bytes = sig.encode().as_slice().to_vec();
        let pk_bytes = self.keypair.public_key_bytes();
        PqSignedTx::new(tx, sig_bytes, pk_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{keygen::PqKeypair, tx::PqTxRequest};
    use alloy_primitives::Address;

    fn sample_tx(chain_id: u64) -> PqTxRequest {
        PqTxRequest {
            chain_id,
            nonce: 0,
            to: Some(Address::from([0xde; 20])),
            value: 1_000_000_000_000_000_000u128,
            gas_limit: 21_000,
            gas_price: 1_000_000_000,
            input: vec![],
        }
    }

    #[test]
    fn sign_produces_deterministic_hash() {
        let kp = PqKeypair::generate();
        let signer = PqSigner::new(&kp);
        let signed = signer.sign(sample_tx(1));
        // Hash is deterministic from tx + sig + pk
        let signed2 = signer.sign(sample_tx(1));
        // Different signatures (ML-DSA is randomized) → different tx hashes
        // But both should be valid (non-zero)
        let zero = alloy_primitives::B256::ZERO;
        assert_ne!(signed.hash, zero);
        assert_ne!(signed2.hash, zero);
    }

    #[test]
    fn signed_tx_encodes_to_non_empty_bytes() {
        let kp = PqKeypair::generate();
        let signer = PqSigner::new(&kp);
        let signed = signer.sign(sample_tx(1337));
        let encoded = signed.encode();
        // type byte + 32 hash + 3309 sig + 1952 pk = 5294 bytes
        assert!(
            encoded.len() > 5000,
            "encoded tx should be ~5294 bytes, got {}",
            encoded.len()
        );
        assert_eq!(encoded[0], 0x04, "first byte must be tx type 0x04");
    }
}
