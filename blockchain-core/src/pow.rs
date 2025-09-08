use sha2::{Sha256, Digest};
use crate::block::Block;

pub struct ProofOfWork {
    pub block: Block,
    pub difficulty: usize,
}

impl ProofOfWork {
    pub fn new(block: Block, difficulty: usize) -> Self {
        ProofOfWork { block, difficulty }
    }

    pub fn mine_block(&mut self) -> (u64, String) {
        let target = "0".repeat(self.difficulty);
        let mut nonce = 0u64;
        loop {
            let hash = self.calculate_hash(nonce);
            if &hash[..self.difficulty] == target {
                self.block.nonce = nonce;
                self.block.hash = hash.clone();
                return (nonce, hash);
            }
            nonce += 1;
        }
    }

    fn calculate_hash(&self, nonce: u64) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}{}{}{}{}{}", 
            self.block.index, 
            self.block.timestamp, 
            &self.block.data, 
            &self.block.previous_hash, 
            nonce,
            self.block.difficulty
        ));
        format!("{:x}", hasher.finalize())
    }

    pub fn validate(&self) -> bool {
        let hash = self.calculate_hash(self.block.nonce);
        let target = "0".repeat(self.difficulty);
        &hash[..self.difficulty] == target && hash == self.block.hash
    }
}