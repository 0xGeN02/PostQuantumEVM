use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub timestamp: i64,
    pub data: String,
    pub previous_hash: String,
    pub hash: String,
    pub nonce: u64,
    pub difficulty: usize,
    pub consensus_data: HashMap<String, String>, // Datos específicos del consenso
}

impl Block {
    pub fn new(index: u64, data: String, previous_hash: String) -> Self {
        let timestamp = Utc::now().timestamp();
        Block {
            index,
            timestamp,
            data,
            previous_hash,
            hash: String::new(), // Will be calculated by consensus algorithm
            nonce: 0,
            difficulty: 4, // Default value for compatibility
            consensus_data: HashMap::new(),
        }
    }

    /// Constructor legacy que mantiene compatibilidad con difficulty
    pub fn new_with_difficulty(
        index: u64,
        data: String,
        previous_hash: String,
        difficulty: usize,
    ) -> Self {
        let timestamp = Utc::now().timestamp();
        Block {
            index,
            timestamp,
            data,
            previous_hash,
            hash: String::new(),
            nonce: 0,
            difficulty,
            consensus_data: HashMap::new(),
        }
    }

    /// Calcula un hash básico del bloque (usado principalmente para validación)
    pub fn calculate_basic_hash(&self) -> String {
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

    /// Método legacy para compatibilidad
    pub fn calculate_hash(&self) -> String {
        self.calculate_basic_hash()
    }

    /// Método legacy para compatibilidad (ahora obsoleto)
    #[deprecated(note = "Use consensus algorithm's execute_consensus method instead")]
    pub fn mine_block(&mut self) {
        // Por compatibilidad, implementa PoW básico
        use crate::consensus::{ConsensusAlgorithm, ProofOfWork};
        let mut pow = ProofOfWork::new(self.difficulty);
        if let Ok(result) = pow.execute_consensus(self) {
            self.hash = result.block.hash;
            self.nonce = result.block.nonce;
            self.consensus_data = result.proof_data;
        }
    }

    /// Actualiza los datos de consenso del bloque
    pub fn set_consensus_data(&mut self, data: HashMap<String, String>) {
        self.consensus_data = data;
    }

    /// Obtiene un valor específico de los datos de consenso
    pub fn get_consensus_data(&self, key: &str) -> Option<&String> {
        self.consensus_data.get(key)
    }

    /// Verifica si el bloque tiene datos de un tipo específico de consenso
    pub fn has_consensus_type(&self, consensus_type: &str) -> bool {
        self.consensus_data.contains_key("algorithm_name")
            && self.consensus_data.get("algorithm_name") == Some(&consensus_type.to_string())
    }
}
