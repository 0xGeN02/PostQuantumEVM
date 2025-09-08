use sha2::{Sha256, Digest};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use crate::pow::ProofOfWork;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub timestamp: i64,
    pub data: String,
    pub previous_hash: String,
    pub hash: String,
    pub nonce: u64,
    pub difficulty: usize,
}

impl Block {
    pub fn new(index: u64, data: String, previous_hash: String, difficulty: usize) -> Self {
        let timestamp = Utc::now().timestamp();
        Block {
            index,
            timestamp,
            data,
            previous_hash,
            hash: String::new(), // Calculated via PoW
            nonce: 0,
            difficulty
        }
    }

    pub fn mine_block(&mut self) {
        let mut pow = ProofOfWork::new(self.clone(), self.difficulty);
        let (nonce, hash) = pow.mine_block();
        self.nonce = nonce;
        self.hash = hash;
    }

    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}{}{}{}{}{}", 
            self.index, 
            self.timestamp, 
            &self.data, 
            &self.previous_hash, 
            self.nonce,
            self.difficulty
        ));
        format!("{:x}", hasher.finalize())
    }
}