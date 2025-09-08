use crate::block::Block;
use crate::consensus::traits::{ConsensusAlgorithm, ConsensusConfig, ConsensusResult};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Proof of Elapsed Time - Simulación de Intel SGX
#[derive(Debug, Clone)]
pub struct ProofOfElapsedTime {
    pub wait_time_config: u64, // Tiempo base de espera en millisegundos
    pub node_id: String,
    pub trusted_execution: bool, // Simulación de SGX
}

impl ProofOfElapsedTime {
    pub fn new(wait_time_config: u64, node_id: String) -> Self {
        ProofOfElapsedTime {
            wait_time_config,
            node_id,
            trusted_execution: true, // Simulamos que tenemos SGX
        }
    }

    /// Genera un tiempo de espera aleatorio usando el hash del bloque anterior
    fn generate_wait_time(&self, block: &Block) -> (Duration, String) {
        // Crear semilla determinística basada en el bloque anterior y el nodo
        let seed_input = format!("{}{}{}", block.previous_hash, self.node_id, block.index);

        let mut hasher = Sha256::new();
        hasher.update(seed_input.as_bytes());
        let hash_result = hasher.finalize();

        // Convertir hash a semilla
        let mut seed_bytes = [0u8; 32];
        seed_bytes.copy_from_slice(&hash_result);

        let mut rng = StdRng::from_seed(seed_bytes);

        // Generar tiempo de espera aleatorio (0.5x a 2x el tiempo configurado)
        let multiplier = rng.random_range(0.5..2.0);
        let wait_time_ms = (self.wait_time_config as f64 * multiplier) as u64;
        let wait_duration = Duration::from_millis(wait_time_ms);

        // Crear "certificado" de tiempo de espera
        let certificate = format!("{:x}", hash_result);

        (wait_duration, certificate)
    }

    /// Simula la espera en un entorno de ejecución confiable
    fn trusted_wait(&self, wait_time: Duration) -> Result<String, String> {
        if !self.trusted_execution {
            return Err("Trusted execution environment not available".to_string());
        }

        // En implementación real, esto sería manejado por Intel SGX
        std::thread::sleep(wait_time);

        // Generar prueba de tiempo transcurrido
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}{}{}",
            self.node_id,
            wait_time.as_millis(),
            chrono::Utc::now().timestamp()
        ));

        Ok(format!("{:x}", hasher.finalize()))
    }

    fn create_poet_proof(&self, block: &Block) -> Result<(Duration, String, String), String> {
        let (wait_time, certificate) = self.generate_wait_time(block);
        let elapsed_proof = self.trusted_wait(wait_time)?;

        Ok((wait_time, certificate, elapsed_proof))
    }

    fn verify_poet_proof(&self, block: &Block, certificate: &str, elapsed_proof: &str) -> bool {
        // En implementación real, esto verificaría la firma SGX
        // Por ahora, verificación básica

        // Verificar que el certificado es válido para este bloque y nodo
        let seed_input = format!("{}{}{}", block.previous_hash, self.node_id, block.index);

        let mut hasher = Sha256::new();
        hasher.update(seed_input.as_bytes());
        let expected_certificate = format!("{:x}", hasher.finalize());

        certificate == expected_certificate && elapsed_proof.len() == 64 // Longitud de hash SHA256
    }
}

impl ConsensusAlgorithm for ProofOfElapsedTime {
    fn execute_consensus(&mut self, block: &mut Block) -> Result<ConsensusResult, String> {
        let start_time = Instant::now();

        if !self.trusted_execution {
            return Err("Trusted execution environment required for PoET".to_string());
        }

        // Generar y ejecutar prueba de tiempo transcurrido
        let (wait_time, certificate, elapsed_proof) = self.create_poet_proof(block)?;

        // Crear hash del bloque incluyendo la prueba PoET
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}{}{}{}{}{}{}",
            block.index,
            block.timestamp,
            &block.data,
            &block.previous_hash,
            &certificate,
            &elapsed_proof,
            &self.node_id
        ));

        block.hash = format!("{:x}", hasher.finalize());
        block.nonce = wait_time.as_millis() as u64; // Usar tiempo de espera como nonce

        let total_duration = start_time.elapsed();

        // Preparar datos de prueba
        let mut proof_data = HashMap::new();
        proof_data.insert("node_id".to_string(), self.node_id.clone());
        proof_data.insert(
            "wait_time_ms".to_string(),
            wait_time.as_millis().to_string(),
        );
        proof_data.insert("wait_certificate".to_string(), certificate);
        proof_data.insert("elapsed_proof".to_string(), elapsed_proof);
        proof_data.insert(
            "trusted_execution".to_string(),
            self.trusted_execution.to_string(),
        );

        Ok(ConsensusResult {
            block: block.clone(),
            proof_data,
            execution_time: total_duration,
            energy_cost: Some(0.001), // Muy bajo consumo (principalmente espera)
        })
    }

    fn validate_block(&self, block: &Block) -> bool {
        // Extraer datos de la prueba del hash (simplificado)
        // En implementación real, estos datos estarían en el bloque

        // Validación básica: verificar que el nonce (tiempo de espera) es razonable
        let wait_time_ms = block.nonce;
        let min_wait = (self.wait_time_config as f64 * 0.5) as u64;
        let max_wait = (self.wait_time_config as f64 * 2.0) as u64;

        if wait_time_ms < min_wait || wait_time_ms > max_wait {
            return false;
        }

        // En implementación real, verificaríamos la firma SGX
        block.hash.len() == 64 // Verificación básica de hash SHA256
    }

    fn get_algorithm_name(&self) -> &'static str {
        "Proof of Elapsed Time"
    }

    fn get_energy_efficiency(&self) -> Option<f64> {
        Some(0.98) // Muy alta eficiencia (principalmente tiempo de espera)
    }

    fn get_statistics(&self) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        stats.insert("node_id".to_string(), self.node_id.clone());
        stats.insert(
            "wait_time_config_ms".to_string(),
            self.wait_time_config.to_string(),
        );
        stats.insert(
            "trusted_execution".to_string(),
            self.trusted_execution.to_string(),
        );
        stats.insert("algorithm_type".to_string(), "lottery_system".to_string());

        // Estadísticas de tiempo de espera
        let min_wait = (self.wait_time_config as f64 * 0.5) as u64;
        let max_wait = (self.wait_time_config as f64 * 2.0) as u64;
        stats.insert("min_wait_time_ms".to_string(), min_wait.to_string());
        stats.insert("max_wait_time_ms".to_string(), max_wait.to_string());

        stats
    }

    fn configure(&mut self, config: ConsensusConfig) -> Result<(), String> {
        if let Some(wait_time_str) = config.additional_params.get("wait_time_config") {
            self.wait_time_config = wait_time_str
                .parse()
                .map_err(|_| "Invalid wait_time_config parameter".to_string())?;
        }

        if let Some(node_id) = config.additional_params.get("node_id") {
            self.node_id = node_id.clone();
        }

        if let Some(trusted_str) = config.additional_params.get("trusted_execution") {
            self.trusted_execution = trusted_str
                .parse()
                .map_err(|_| "Invalid trusted_execution parameter".to_string())?;
        }

        Ok(())
    }
}
