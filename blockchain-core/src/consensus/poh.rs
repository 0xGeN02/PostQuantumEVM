use crate::block::Block;
use crate::consensus::traits::{ConsensusAlgorithm, ConsensusConfig, ConsensusResult};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Función de Retardo Verificable simplificada (VDF)
#[derive(Debug, Clone)]
pub struct ProofOfHistory {
    pub vdf_iterations: u64,
    pub sequence_number: u64,
    pub previous_output: String,
}

impl ProofOfHistory {
    pub fn new(vdf_iterations: u64) -> Self {
        ProofOfHistory {
            vdf_iterations,
            sequence_number: 0,
            previous_output: "genesis".to_string(),
        }
    }

    /// Función VDF simplificada - En implementación real sería más compleja
    fn compute_vdf(&self, input: &str, iterations: u64) -> (String, Duration) {
        let start_time = Instant::now();
        let mut current = input.to_string();

        for _ in 0..iterations {
            let mut hasher = Sha256::new();
            hasher.update(current.as_bytes());
            current = format!("{:x}", hasher.finalize());
        }

        (current, start_time.elapsed())
    }

    fn create_history_proof(&mut self, block: &Block) -> (String, u64, Duration) {
        // Crear entrada para VDF
        let input = format!(
            "{}{}{}{}{}",
            self.previous_output, block.index, block.timestamp, block.data, block.previous_hash
        );

        let (output, duration) = self.compute_vdf(&input, self.vdf_iterations);
        self.sequence_number += 1;
        self.previous_output = output.clone();

        (output, self.sequence_number, duration)
    }

    fn verify_history_proof(&self, block: &Block, claimed_output: &str, sequence: u64) -> bool {
        // En implementación real, esto sería más sofisticado
        let input = format!(
            "{}{}{}{}{}",
            // Necesitaríamos el output previo almacenado
            claimed_output, // Simplificación
            block.index,
            block.timestamp,
            block.data,
            block.previous_hash
        );

        let (expected_output, _) = self.compute_vdf(&input, self.vdf_iterations);
        expected_output == *claimed_output && sequence > 0
    }
}

impl ConsensusAlgorithm for ProofOfHistory {
    fn execute_consensus(&mut self, block: &mut Block) -> Result<ConsensusResult, String> {
        let start_time = Instant::now();

        // Crear prueba de historia
        let (history_output, sequence, vdf_duration) = self.create_history_proof(block);

        // El hash del bloque incluye la prueba de historia
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}{}{}{}{}{}",
            block.index,
            block.timestamp,
            &block.data,
            &block.previous_hash,
            &history_output,
            sequence
        ));

        block.hash = format!("{:x}", hasher.finalize());
        block.nonce = sequence; // Usamos el número de secuencia como nonce

        let total_duration = start_time.elapsed();

        // Preparar datos de prueba
        let mut proof_data = HashMap::new();
        proof_data.insert("history_output".to_string(), history_output);
        proof_data.insert("sequence_number".to_string(), sequence.to_string());
        proof_data.insert(
            "vdf_iterations".to_string(),
            self.vdf_iterations.to_string(),
        );
        proof_data.insert(
            "vdf_duration_ms".to_string(),
            vdf_duration.as_millis().to_string(),
        );

        Ok(ConsensusResult {
            block: block.clone(),
            proof_data,
            execution_time: total_duration,
            energy_cost: Some(0.01), // Relativamente bajo consumo
        })
    }

    fn validate_block(&self, block: &Block) -> bool {
        // En implementación real, necesitaríamos acceso al estado histórico
        // Por ahora, validación básica
        if block.nonce == 0 {
            return false;
        }

        // Verificar que el hash es consistente
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}{}{}{}{}{}",
            block.index,
            block.timestamp,
            &block.data,
            &block.previous_hash,
            "placeholder_history", // En implementación real sería el output real
            block.nonce
        ));

        let expected_hash = format!("{:x}", hasher.finalize());
        expected_hash == block.hash
    }

    fn get_algorithm_name(&self) -> &'static str {
        "Proof of History"
    }

    fn get_energy_efficiency(&self) -> Option<f64> {
        Some(0.85) // Buena eficiencia energética
    }

    fn get_statistics(&self) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        stats.insert(
            "vdf_iterations".to_string(),
            self.vdf_iterations.to_string(),
        );
        stats.insert(
            "current_sequence".to_string(),
            self.sequence_number.to_string(),
        );
        stats.insert(
            "algorithm_type".to_string(),
            "cryptographic_clock".to_string(),
        );

        // Estimar tiempo por VDF
        let estimated_time_per_vdf = (self.vdf_iterations as f64) * 0.001; // ms por iteración
        stats.insert(
            "estimated_vdf_time_ms".to_string(),
            estimated_time_per_vdf.to_string(),
        );

        stats
    }

    fn configure(&mut self, config: ConsensusConfig) -> Result<(), String> {
        if let Some(iterations_str) = config.additional_params.get("vdf_iterations") {
            self.vdf_iterations = iterations_str
                .parse()
                .map_err(|_| "Invalid vdf_iterations parameter".to_string())?;
        }

        if let Some(seq_str) = config.additional_params.get("reset_sequence") {
            if seq_str == "true" {
                self.sequence_number = 0;
                self.previous_output = "genesis".to_string();
            }
        }

        Ok(())
    }
}
