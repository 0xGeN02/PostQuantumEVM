//! # Blockchain Core with Configurable Consensus Algorithms
//!
//! This library provides a flexible blockchain implementation that supports multiple
//! consensus algorithms including:
//!
//! - **Proof of Work (PoW)**: Traditional computational proof requiring miners to solve cryptographic puzzles
//! - **Proof of Stake (PoS)**: Stake-based selection where validators are chosen based on their holdings  
//! - **Proof of Authority (PoA)**: Identity-based consensus with pre-approved validators
//! - **Proof of History (PoH)**: Cryptographic clock creating verifiable passage of time
//! - **Proof of Elapsed Time (PoET)**: Random lottery system using trusted execution environments
//! - **Proof of Burn (PoB)**: Proof of destroyed coins to gain mining rights
//! - **Proof of Capacity (PoC)**: Storage-based proof using pre-computed hash tables
//! - **Practical Byzantine Fault Tolerance (pBFT)**: Byzantine fault tolerant consensus for permissioned networks
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use blockchain_core::{Blockchain, ConsensusType};
//!
//! // Create a blockchain with Proof of Work
//! let mut blockchain = Blockchain::new_with_consensus(
//!     ConsensusType::ProofOfWork { difficulty: 4 }
//! ).expect("Failed to create blockchain");
//!
//! // Add a block
//! blockchain.add_block("Alice pays Bob 10 coins".to_string())
//!     .expect("Failed to add block");
//!
//! // Switch to a different consensus algorithm
//! blockchain.switch_consensus(
//!     ConsensusType::ProofOfStake { minimum_stake: 1000 }
//! ).expect("Failed to switch consensus");
//!
//! // Validate the blockchain
//! assert!(blockchain.is_valid());
//! ```
//!
//! ## Features
//!
//! - **Modular Architecture**: Easy to add new consensus algorithms
//! - **Dynamic Consensus Switching**: Change consensus algorithms at runtime
//! - **Performance Metrics**: Built-in benchmarking and statistics
//! - **Energy Efficiency Analysis**: Compare energy consumption across algorithms
//! - **Comprehensive Logging**: Detailed logging for analysis and debugging

pub mod block;
pub mod blockchain;
pub mod consensus;
pub mod logger;

// Re-export main types for convenience
pub use block::Block;
pub use blockchain::{Blockchain, BlockchainStats};
pub use consensus::{
    ConsensusAlgorithm, ConsensusConfig, ConsensusFactory, ConsensusResult, ConsensusType,
    PracticalByzantineFaultTolerance, ProofOfAuthority, ProofOfBurn, ProofOfCapacity,
    ProofOfElapsedTime, ProofOfHistory, ProofOfStake, ProofOfWork,
};
pub use logger::BlockchainLogger;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Convenience function to create a new blockchain with default PoW consensus
pub fn new_blockchain() -> Blockchain {
    Blockchain::new()
}

/// Convenience function to create a blockchain with specific consensus
pub fn new_blockchain_with_consensus(consensus_type: ConsensusType) -> Result<Blockchain, String> {
    Blockchain::new_with_consensus(consensus_type)
}

/// Get information about all available consensus algorithms
pub fn list_consensus_algorithms() -> Vec<(String, String)> {
    vec![
        (
            "Proof of Work".to_string(),
            "Computational proof requiring miners to solve cryptographic puzzles".to_string(),
        ),
        (
            "Proof of Stake".to_string(),
            "Stake-based selection where validators are chosen based on their holdings".to_string(),
        ),
        (
            "Proof of Authority".to_string(),
            "Identity-based consensus with pre-approved validators".to_string(),
        ),
        (
            "Proof of History".to_string(),
            "Cryptographic clock creating verifiable passage of time".to_string(),
        ),
        (
            "Proof of Elapsed Time".to_string(),
            "Random lottery system using trusted execution environments".to_string(),
        ),
        (
            "Proof of Burn".to_string(),
            "Proof of destroyed coins to gain mining rights".to_string(),
        ),
        (
            "Proof of Capacity".to_string(),
            "Storage-based proof using pre-computed hash tables".to_string(),
        ),
        (
            "Practical Byzantine Fault Tolerance".to_string(),
            "Byzantine fault tolerant consensus for permissioned networks".to_string(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blockchain_creation() {
        let blockchain = new_blockchain();
        assert_eq!(blockchain.blocks.len(), 1); // Genesis block
        assert!(blockchain.is_valid());
    }

    #[test]
    fn test_consensus_switching() {
        let mut blockchain = new_blockchain();

        // Add block with PoW
        blockchain
            .add_block("Test transaction".to_string())
            .unwrap();

        // Switch to PoS
        blockchain
            .switch_consensus(ConsensusType::ProofOfStake {
                minimum_stake: 1000,
            })
            .unwrap();

        // Add block with PoS
        blockchain
            .add_block("Test transaction 2".to_string())
            .unwrap();

        assert_eq!(blockchain.blocks.len(), 3); // Genesis + 2 blocks
        assert!(blockchain.is_valid());
    }

    #[test]
    fn test_all_consensus_algorithms() {
        let algorithms = vec![
            ConsensusType::ProofOfWork { difficulty: 2 },
            ConsensusType::ProofOfStake { minimum_stake: 100 },
            ConsensusType::ProofOfAuthority {
                validators: vec!["test".to_string()],
            },
            ConsensusType::ProofOfHistory { vdf_iterations: 10 },
            ConsensusType::ProofOfElapsedTime {
                wait_time_config: 10,
            },
            ConsensusType::ProofOfBurn { burn_amount: 10 },
            ConsensusType::ProofOfCapacity {
                storage_requirement: 1,
            },
            ConsensusType::PracticalByzantineFaultTolerance {
                node_count: 4,
                fault_tolerance: 0.25,
            },
        ];

        for consensus_type in algorithms {
            let result = new_blockchain_with_consensus(consensus_type);
            assert!(
                result.is_ok(),
                "Failed to create blockchain with consensus algorithm"
            );
        }
    }
}
