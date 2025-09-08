use crate::block::Block;
use crate::consensus::traits::{ConsensusAlgorithm, ConsensusConfig, ConsensusResult};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ProofOfWork {
    pub difficulty: usize,
    pub target_time: Duration, // Tiempo objetivo entre bloques
}

impl ProofOfWork {
    pub fn new(difficulty: usize) -> Self {
        ProofOfWork {
            difficulty,
            target_time: Duration::from_secs(60), // 1 minuto por defecto
        }
    }

    pub fn with_target_time(difficulty: usize, target_time: Duration) -> Self {
        ProofOfWork {
            difficulty,
            target_time,
        }
    }

    fn mine_block(&self, block: &mut Block) -> (u64, String, Duration) {
        let target = "0".repeat(self.difficulty);
        let mut nonce = 0u64;
        let start_time = Instant::now();

        loop {
            let hash = self.calculate_hash(block, nonce);
            if &hash[..self.difficulty] == target {
                let duration = start_time.elapsed();
                return (nonce, hash, duration);
            }
            nonce += 1;

            // Prevenir loops infinitos en dificultades muy altas
            if nonce % 1_000_000 == 0 {
                println!("PoW: Minado en progreso, nonce: {}", nonce);
            }
        }
    }

    fn calculate_hash(&self, block: &Block, nonce: u64) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}{}{}{}{}{}",
            block.index, block.timestamp, &block.data, &block.previous_hash, nonce, self.difficulty
        ));
        format!("{:x}", hasher.finalize())
    }
}

impl ConsensusAlgorithm for ProofOfWork {
    fn execute_consensus(&mut self, block: &mut Block) -> Result<ConsensusResult, String> {
        let (nonce, hash, duration) = self.mine_block(block);

        // Actualizar el bloque
        block.nonce = nonce;
        block.hash = hash.clone();
        block.difficulty = self.difficulty;

        // Preparar datos de prueba
        let mut proof_data = HashMap::new();
        proof_data.insert("nonce".to_string(), nonce.to_string());
        proof_data.insert("difficulty".to_string(), self.difficulty.to_string());
        proof_data.insert("target".to_string(), "0".repeat(self.difficulty));

        // Estimar costo energético (muy básico)
        let energy_cost = (nonce as f64) * 0.0001; // Estimación simplificada

        Ok(ConsensusResult {
            block: block.clone(),
            proof_data,
            execution_time: duration,
            energy_cost: Some(energy_cost),
        })
    }

    fn validate_block(&self, block: &Block) -> bool {
        let hash = self.calculate_hash(block, block.nonce);
        let target = "0".repeat(self.difficulty);
        &hash[..self.difficulty] == target && hash == block.hash
    }

    fn get_algorithm_name(&self) -> &'static str {
        "Proof of Work"
    }

    fn calculate_next_difficulty(&self, blocks: &[Block]) -> Option<usize> {
        if blocks.len() < 2 {
            return Some(self.difficulty);
        }

        // Ajuste de dificultad basado en tiempo promedio de bloques
        let recent_blocks = &blocks[blocks.len().saturating_sub(10)..];
        let total_time: i64 = recent_blocks
            .windows(2)
            .map(|pair| pair[1].timestamp - pair[0].timestamp)
            .sum();

        let avg_time = total_time / (recent_blocks.len() - 1) as i64;
        let target_seconds = self.target_time.as_secs() as i64;

        if avg_time < target_seconds / 2 {
            Some(self.difficulty + 1) // Aumentar dificultad
        } else if avg_time > target_seconds * 2 {
            Some(self.difficulty.saturating_sub(1)) // Disminuir dificultad
        } else {
            Some(self.difficulty) // Mantener dificultad
        }
    }

    fn get_energy_efficiency(&self) -> Option<f64> {
        Some(1.0 / (self.difficulty as f64).powf(2.0)) // Menor eficiencia con mayor dificultad
    }

    fn get_statistics(&self) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        stats.insert("difficulty".to_string(), self.difficulty.to_string());
        stats.insert(
            "target_time_seconds".to_string(),
            self.target_time.as_secs().to_string(),
        );
        stats.insert(
            "algorithm_type".to_string(),
            "computational_proof".to_string(),
        );
        stats
    }

    fn configure(&mut self, config: ConsensusConfig) -> Result<(), String> {
        if let Some(difficulty_str) = config.additional_params.get("difficulty") {
            self.difficulty = difficulty_str
                .parse()
                .map_err(|_| "Invalid difficulty parameter".to_string())?;
        }

        if let Some(target_time_str) = config.additional_params.get("target_time_seconds") {
            let seconds: u64 = target_time_str
                .parse()
                .map_err(|_| "Invalid target_time parameter".to_string())?;
            self.target_time = Duration::from_secs(seconds);
        }

        Ok(())
    }
}
