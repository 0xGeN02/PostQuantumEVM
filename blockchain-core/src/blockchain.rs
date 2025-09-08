use crate::block::Block;
use crate::pow::ProofOfWork;
use crate::logger::BlockchainLogger;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub difficulty: usize,
    #[serde(skip)]
    pub logger: Option<BlockchainLogger>,
}

impl Blockchain {
    pub fn new() -> Self {
        let logger = BlockchainLogger::new();
        let mut genesis_block = Block::new(0, "Genesis Block".to_string(), "0".to_string(), 2);
        genesis_block.mine_block(); 
        
        let blockchain = Blockchain {
            blocks: vec![genesis_block.clone()], 
            difficulty: 4,
            logger: Some(logger),
        };

        // Log the creation of the genesis block
        if let Some(ref logger) = blockchain.logger {
            logger.log_block_creation(&genesis_block);
        }

        blockchain
    }

    pub fn add_block(&mut self, data: String) {
        let previous_hash = self.blocks.last().unwrap().hash.clone();
        let current_difficulty = self.calculate_next_difficulty();
        let mut new_block = Block::new(
            self.blocks.len() as u64, 
            data, 
            previous_hash,
            current_difficulty
        );
        
        // Log mining start
        if let Some(ref logger) = self.logger {
            logger.log_mining_start(new_block.index, current_difficulty);
        }
        
        let start = std::time::Instant::now();
        new_block.mine_block();
        let duration = start.elapsed();

        // Log mining completion
        if let Some(ref logger) = self.logger {
            logger.log_mining_complete(&new_block, duration);
            logger.log_block_creation(&new_block);
        }

        self.blocks.push(new_block);
    }

    pub fn is_valid(&self) -> bool {
        for i in 1..self.blocks.len() {
            let current = &self.blocks[i];
            let previous = &self.blocks[i - 1];
            if current.previous_hash != previous.hash {
                return false;
            }
            if current.hash != current.calculate_hash() {
                return false;
            }
            let pow = ProofOfWork::new(current.clone(), self.difficulty);
            if !pow.validate() {
                return false;
            }
        }
        true
    }


    fn calculate_next_difficulty(&self) -> usize {
        // Simple example: increase difficulty every 2 blocks
        if self.blocks.len() % 2 == 0 && self.blocks.len() > 0 {
            let last_difficulty = self.blocks.last().unwrap().difficulty;
            std::cmp::min(last_difficulty + 1, 6) // Cap at difficulty 6
        } else {
            self.difficulty
        }
    }

    pub fn get_difficulty_stats(&self) -> (usize, usize, f64) {
        let difficulties: Vec<usize> = self.blocks.iter().map(|b| b.difficulty).collect();
        let min_diff = *difficulties.iter().min().unwrap_or(&0);
        let max_diff = *difficulties.iter().max().unwrap_or(&0);
        let avg_diff = difficulties.iter().sum::<usize>() as f64 / difficulties.len() as f64;
        (min_diff, max_diff, avg_diff)
    }

    pub fn log_blockchain_state(&self) {
        if let Some(ref logger) = self.logger {
            logger.log_blockchain_state(self);
        }
    }

    pub fn log_validation_result(&self) -> bool {
        let is_valid = self.is_valid();
        if let Some(ref logger) = self.logger {
            logger.log_validation_result(is_valid);
        }
        is_valid
    }

    pub fn log_difficulty_stats(&self) {
        let (min_diff, max_diff, avg_diff) = self.get_difficulty_stats();
        if let Some(ref logger) = self.logger {
            logger.log_difficulty_stats(min_diff, max_diff, avg_diff);
        }
    }

    pub fn create_summary_report(&self) {
        if let Some(ref logger) = self.logger {
            logger.create_summary_report(self);
        }
    }
}