pub mod pbft;
pub mod poa;
pub mod pob;
pub mod poc;
pub mod poet;
pub mod poh;
pub mod pos;
pub mod pow;
pub mod traits;

pub use pbft::PracticalByzantineFaultTolerance;
pub use poa::ProofOfAuthority;
pub use pob::ProofOfBurn;
pub use poc::ProofOfCapacity;
pub use poet::ProofOfElapsedTime;
pub use poh::ProofOfHistory;
pub use pos::ProofOfStake;
pub use pow::ProofOfWork;
pub use traits::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Factory para crear algoritmos de consenso
pub struct ConsensusFactory;

impl ConsensusFactory {
    pub fn create_consensus(
        consensus_type: &ConsensusType,
    ) -> Result<Box<dyn ConsensusAlgorithm>, String> {
        match consensus_type {
            ConsensusType::ProofOfWork { difficulty } => {
                Ok(Box::new(ProofOfWork::new(*difficulty)))
            }
            ConsensusType::ProofOfStake { minimum_stake } => {
                Ok(Box::new(ProofOfStake::new(*minimum_stake)))
            }
            ConsensusType::ProofOfHistory { vdf_iterations } => {
                Ok(Box::new(ProofOfHistory::new(*vdf_iterations)))
            }
            ConsensusType::ProofOfAuthority { validators } => {
                Ok(Box::new(ProofOfAuthority::new(validators.clone())))
            }
            ConsensusType::ProofOfElapsedTime { wait_time_config } => {
                let node_id = format!("node_{}", rand::random::<u32>());
                Ok(Box::new(ProofOfElapsedTime::new(
                    *wait_time_config,
                    node_id,
                )))
            }
            ConsensusType::ProofOfBurn { burn_amount } => {
                Ok(Box::new(ProofOfBurn::new(*burn_amount)))
            }
            ConsensusType::ProofOfCapacity {
                storage_requirement,
            } => Ok(Box::new(ProofOfCapacity::new(*storage_requirement))),
            ConsensusType::PracticalByzantineFaultTolerance {
                node_count,
                fault_tolerance,
            } => Ok(Box::new(PracticalByzantineFaultTolerance::new(
                *node_count,
                *fault_tolerance,
            ))),
        }
    }
}

/// Enumeración para seleccionar el tipo de consenso
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusType {
    ProofOfWork {
        difficulty: usize,
    },
    ProofOfStake {
        minimum_stake: u64,
    },
    ProofOfHistory {
        vdf_iterations: u64,
    },
    ProofOfAuthority {
        validators: Vec<String>,
    },
    ProofOfElapsedTime {
        wait_time_config: u64,
    },
    ProofOfBurn {
        burn_amount: u64,
    },
    ProofOfCapacity {
        storage_requirement: u64,
    },
    PracticalByzantineFaultTolerance {
        node_count: usize,
        fault_tolerance: f32,
    },
}

impl Default for ConsensusType {
    fn default() -> Self {
        ConsensusType::ProofOfWork { difficulty: 4 }
    }
}

impl ConsensusType {
    /// Devuelve el nombre del algoritmo de consenso
    pub fn name(&self) -> &'static str {
        match self {
            ConsensusType::ProofOfWork { .. } => "Proof of Work",
            ConsensusType::ProofOfStake { .. } => "Proof of Stake",
            ConsensusType::ProofOfHistory { .. } => "Proof of History",
            ConsensusType::ProofOfAuthority { .. } => "Proof of Authority",
            ConsensusType::ProofOfElapsedTime { .. } => "Proof of Elapsed Time",
            ConsensusType::ProofOfBurn { .. } => "Proof of Burn",
            ConsensusType::ProofOfCapacity { .. } => "Proof of Capacity",
            ConsensusType::PracticalByzantineFaultTolerance { .. } => {
                "Practical Byzantine Fault Tolerance"
            }
        }
    }

    /// Devuelve una descripción del algoritmo
    pub fn description(&self) -> &'static str {
        match self {
            ConsensusType::ProofOfWork { .. } => {
                "Computational proof requiring miners to solve cryptographic puzzles"
            }
            ConsensusType::ProofOfStake { .. } => {
                "Stake-based selection where validators are chosen based on their holdings"
            }
            ConsensusType::ProofOfHistory { .. } => {
                "Cryptographic clock creating verifiable passage of time"
            }
            ConsensusType::ProofOfAuthority { .. } => {
                "Identity-based consensus with pre-approved validators"
            }
            ConsensusType::ProofOfElapsedTime { .. } => {
                "Random lottery system using trusted execution environments"
            }
            ConsensusType::ProofOfBurn { .. } => "Proof of destroyed coins to gain mining rights",
            ConsensusType::ProofOfCapacity { .. } => {
                "Storage-based proof using pre-computed hash tables"
            }
            ConsensusType::PracticalByzantineFaultTolerance { .. } => {
                "Byzantine fault tolerant consensus for permissioned networks"
            }
        }
    }

    /// Devuelve las características principales del algoritmo
    pub fn characteristics(&self) -> HashMap<&'static str, String> {
        let mut chars = HashMap::new();

        match self {
            ConsensusType::ProofOfWork { difficulty } => {
                chars.insert("energy_efficiency", "Low".to_string());
                chars.insert("security", "High".to_string());
                chars.insert("decentralization", "High".to_string());
                chars.insert("difficulty", difficulty.to_string());
            }
            ConsensusType::ProofOfStake { minimum_stake } => {
                chars.insert("energy_efficiency", "High".to_string());
                chars.insert("security", "High".to_string());
                chars.insert("decentralization", "Medium".to_string());
                chars.insert("minimum_stake", minimum_stake.to_string());
            }
            ConsensusType::ProofOfHistory { vdf_iterations } => {
                chars.insert("energy_efficiency", "Medium".to_string());
                chars.insert("security", "High".to_string());
                chars.insert("decentralization", "Medium".to_string());
                chars.insert("vdf_iterations", vdf_iterations.to_string());
            }
            ConsensusType::ProofOfAuthority { validators } => {
                chars.insert("energy_efficiency", "Very High".to_string());
                chars.insert("security", "Medium".to_string());
                chars.insert("decentralization", "Low".to_string());
                chars.insert("validator_count", validators.len().to_string());
            }
            ConsensusType::ProofOfElapsedTime { wait_time_config } => {
                chars.insert("energy_efficiency", "High".to_string());
                chars.insert("security", "High".to_string());
                chars.insert("decentralization", "Medium".to_string());
                chars.insert("wait_time_ms", wait_time_config.to_string());
            }
            ConsensusType::ProofOfBurn { burn_amount } => {
                chars.insert("energy_efficiency", "High".to_string());
                chars.insert("security", "Medium".to_string());
                chars.insert("decentralization", "High".to_string());
                chars.insert("burn_amount", burn_amount.to_string());
            }
            ConsensusType::ProofOfCapacity {
                storage_requirement,
            } => {
                chars.insert("energy_efficiency", "High".to_string());
                chars.insert("security", "Medium".to_string());
                chars.insert("decentralization", "High".to_string());
                chars.insert("storage_gb", storage_requirement.to_string());
            }
            ConsensusType::PracticalByzantineFaultTolerance {
                node_count,
                fault_tolerance,
            } => {
                chars.insert("energy_efficiency", "Medium".to_string());
                chars.insert("security", "Very High".to_string());
                chars.insert("decentralization", "Low".to_string());
                chars.insert("node_count", node_count.to_string());
                chars.insert("fault_tolerance", (fault_tolerance * 100.0).to_string());
            }
        }

        chars
    }
}
