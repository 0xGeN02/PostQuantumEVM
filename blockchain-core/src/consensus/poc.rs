use crate::block::Block;
use crate::consensus::traits::{ConsensusAlgorithm, ConsensusConfig, ConsensusResult};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Instant;

/// Representa un "plot" de almacenamiento con datos pre-computados
#[derive(Debug, Clone)]
pub struct StoragePlot {
    pub plot_id: String,
    pub size_gb: u64,
    pub nonce_count: u64,
    pub creation_timestamp: i64,
    pub hash_pairs: Vec<(String, String)>, // Pares de hash pre-computados
}

#[derive(Debug, Clone)]
pub struct ProofOfCapacity {
    pub plots: Vec<StoragePlot>,
    pub storage_requirement: u64,         // GB mínimos requeridos
    pub plot_verification_samples: usize, // Número de muestras para verificar plots
}

impl ProofOfCapacity {
    pub fn new(storage_requirement: u64) -> Self {
        ProofOfCapacity {
            plots: Vec::new(),
            storage_requirement,
            plot_verification_samples: 10,
        }
    }

    /// Crear un nuevo plot de almacenamiento
    pub fn create_plot(&mut self, size_gb: u64, nonce_count: u64) -> Result<String, String> {
        if size_gb < self.storage_requirement {
            return Err(format!(
                "Plot size {} GB is below minimum {} GB",
                size_gb, self.storage_requirement
            ));
        }

        let plot_id = format!("plot_{}", self.plots.len());
        let creation_timestamp = chrono::Utc::now().timestamp();

        // Pre-computar pares de hash (simulación)
        let mut hash_pairs = Vec::new();
        let mut rng = rand::rng();

        for i in 0..nonce_count.min(1000) {
            // Limitar para demo
            let nonce = rng.random::<u64>();
            let hash1 = self.compute_hash(&format!("{}{}{}", plot_id, i, nonce));
            let hash2 = self.compute_hash(&format!("{}{}", hash1, nonce));
            hash_pairs.push((hash1, hash2));
        }

        let plot = StoragePlot {
            plot_id: plot_id.clone(),
            size_gb,
            nonce_count,
            creation_timestamp,
            hash_pairs,
        };

        self.plots.push(plot);
        Ok(plot_id)
    }

    fn compute_hash(&self, input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Buscar el mejor deadlines en todos los plots
    fn find_best_deadline(&self, block: &Block) -> Option<(String, u64, String)> {
        let mut best_deadline = u64::MAX;
        let mut best_plot_id = String::new();
        let mut best_hash = String::new();

        for plot in &self.plots {
            if let Some((deadline, hash)) = self.calculate_deadline_for_plot(plot, block) {
                if deadline < best_deadline {
                    best_deadline = deadline;
                    best_plot_id = plot.plot_id.clone();
                    best_hash = hash;
                }
            }
        }

        if best_deadline == u64::MAX {
            None
        } else {
            Some((best_plot_id, best_deadline, best_hash))
        }
    }

    fn calculate_deadline_for_plot(
        &self,
        plot: &StoragePlot,
        block: &Block,
    ) -> Option<(u64, String)> {
        // Crear "generation signature" basada en el bloque anterior
        let generation_signature = self.compute_hash(&format!(
            "{}{}{}",
            block.previous_hash, block.index, plot.plot_id
        ));

        // Buscar en los hash pairs pre-computados
        let target = &generation_signature[..8]; // Primeros 8 caracteres como target

        for (i, (hash1, hash2)) in plot.hash_pairs.iter().enumerate() {
            if hash1.starts_with(target) || hash2.starts_with(target) {
                // Calcular deadline basado en la posición y el hash
                let base_time = (i + 1) as u64 * 1000; // Base en millisegundos
                let hash_modifier = u64::from_str_radix(&hash1[..8], 16).unwrap_or(1) % 10000;
                let deadline = base_time + hash_modifier;

                return Some((deadline, hash1.clone()));
            }
        }

        // Si no se encuentra match exacto, usar el primer hash pair como fallback
        if let Some((hash1, _)) = plot.hash_pairs.first() {
            let deadline = u64::from_str_radix(&hash1[..8], 16).unwrap_or(1000) % 100000;
            Some((deadline, hash1.clone()))
        } else {
            None
        }
    }

    fn verify_plot_capacity(&self, plot: &StoragePlot) -> bool {
        // Verificaciones básicas del plot
        if plot.size_gb < self.storage_requirement {
            return false;
        }

        if plot.hash_pairs.is_empty() {
            return false;
        }

        // Verificar algunos hash pairs aleatoriamente
        let sample_size = self.plot_verification_samples.min(plot.hash_pairs.len());
        let mut rng = rand::rng();

        for _ in 0..sample_size {
            let index = rng.random_range(0..plot.hash_pairs.len());
            let (hash1, hash2) = &plot.hash_pairs[index];

            // Verificar que los hashes tienen el formato correcto
            if hash1.len() != 64 || hash2.len() != 64 {
                return false;
            }

            // Verificación básica de que hash2 deriva de hash1
            let expected_hash2 = self.compute_hash(&format!("{}{}", hash1, index));
            if expected_hash2 != *hash2 {
                return false;
            }
        }

        true
    }

    fn get_total_capacity(&self) -> u64 {
        self.plots.iter().map(|p| p.size_gb).sum()
    }
}

impl ConsensusAlgorithm for ProofOfCapacity {
    fn execute_consensus(&mut self, block: &mut Block) -> Result<ConsensusResult, String> {
        let start_time = Instant::now();

        if self.plots.is_empty() {
            return Err("No storage plots available for mining".to_string());
        }

        // Verificar capacidad total
        let total_capacity = self.get_total_capacity();
        if total_capacity < self.storage_requirement {
            return Err(format!(
                "Total capacity {} GB is below requirement {} GB",
                total_capacity, self.storage_requirement
            ));
        }

        // Encontrar el mejor deadline
        let (best_plot_id, deadline, winning_hash) = self
            .find_best_deadline(block)
            .ok_or("Unable to find valid deadline in any plot")?;

        // Crear prueba de capacidad
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}{}{}{}{}{}",
            block.index,
            block.timestamp,
            &block.data,
            &block.previous_hash,
            &best_plot_id,
            &winning_hash
        ));

        block.hash = format!("{:x}", hasher.finalize());
        block.nonce = deadline; // Usar deadline como nonce

        let duration = start_time.elapsed();

        // Preparar datos de prueba
        let mut proof_data = HashMap::new();
        proof_data.insert("winning_plot".to_string(), best_plot_id.clone());
        proof_data.insert("deadline".to_string(), deadline.to_string());
        proof_data.insert("winning_hash".to_string(), winning_hash);
        proof_data.insert("total_capacity_gb".to_string(), total_capacity.to_string());
        proof_data.insert("total_plots".to_string(), self.plots.len().to_string());

        // Información del plot ganador
        if let Some(winning_plot) = self.plots.iter().find(|p| p.plot_id == best_plot_id) {
            proof_data.insert("plot_size_gb".to_string(), winning_plot.size_gb.to_string());
            proof_data.insert(
                "plot_nonce_count".to_string(),
                winning_plot.nonce_count.to_string(),
            );
        }

        Ok(ConsensusResult {
            block: block.clone(),
            proof_data,
            execution_time: duration,
            energy_cost: Some(0.01), // Bajo consumo (principalmente I/O de disco)
        })
    }

    fn validate_block(&self, block: &Block) -> bool {
        // Verificar que existe un plot que puede generar este deadline
        let deadline = block.nonce;

        for plot in &self.plots {
            if !self.verify_plot_capacity(plot) {
                continue;
            }

            if let Some((calculated_deadline, winning_hash)) =
                self.calculate_deadline_for_plot(plot, block)
            {
                if calculated_deadline == deadline {
                    // Verificar que el hash del bloque es correcto
                    let mut hasher = Sha256::new();
                    hasher.update(format!(
                        "{}{}{}{}{}{}",
                        block.index,
                        block.timestamp,
                        &block.data,
                        &block.previous_hash,
                        &plot.plot_id,
                        &winning_hash
                    ));

                    let expected_hash = format!("{:x}", hasher.finalize());
                    return expected_hash == block.hash;
                }
            }
        }

        false
    }

    fn get_algorithm_name(&self) -> &'static str {
        "Proof of Capacity"
    }

    fn get_energy_efficiency(&self) -> Option<f64> {
        Some(0.95) // Alta eficiencia (usa almacenamiento en lugar de computación)
    }

    fn get_statistics(&self) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        stats.insert("total_plots".to_string(), self.plots.len().to_string());
        stats.insert(
            "total_capacity_gb".to_string(),
            self.get_total_capacity().to_string(),
        );
        stats.insert(
            "storage_requirement_gb".to_string(),
            self.storage_requirement.to_string(),
        );
        stats.insert(
            "verification_samples".to_string(),
            self.plot_verification_samples.to_string(),
        );

        if !self.plots.is_empty() {
            let avg_plot_size = self.get_total_capacity() / self.plots.len() as u64;
            let max_plot_size = self.plots.iter().map(|p| p.size_gb).max().unwrap_or(0);
            let min_plot_size = self.plots.iter().map(|p| p.size_gb).min().unwrap_or(0);

            stats.insert(
                "average_plot_size_gb".to_string(),
                avg_plot_size.to_string(),
            );
            stats.insert("max_plot_size_gb".to_string(), max_plot_size.to_string());
            stats.insert("min_plot_size_gb".to_string(), min_plot_size.to_string());

            // Estadísticas de hash pairs
            let total_hash_pairs: usize = self.plots.iter().map(|p| p.hash_pairs.len()).sum();
            stats.insert("total_hash_pairs".to_string(), total_hash_pairs.to_string());
        }

        stats
    }

    fn configure(&mut self, config: ConsensusConfig) -> Result<(), String> {
        if let Some(storage_req_str) = config.additional_params.get("storage_requirement") {
            self.storage_requirement = storage_req_str
                .parse()
                .map_err(|_| "Invalid storage_requirement parameter".to_string())?;
        }

        if let Some(samples_str) = config.additional_params.get("verification_samples") {
            self.plot_verification_samples = samples_str
                .parse()
                .map_err(|_| "Invalid verification_samples parameter".to_string())?;
        }

        Ok(())
    }
}
