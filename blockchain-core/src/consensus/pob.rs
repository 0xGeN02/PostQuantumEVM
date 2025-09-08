use crate::block::Block;
use crate::consensus::traits::{ConsensusAlgorithm, ConsensusConfig, ConsensusResult};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct BurnTransaction {
    pub amount: u64,
    pub burn_address: String, // Dirección no gastable (ej: 1111111111111111111114oLvT2)
    pub timestamp: i64,
    pub tx_hash: String,
}

#[derive(Debug, Clone)]
pub struct ProofOfBurn {
    pub burn_transactions: Vec<BurnTransaction>,
    pub burn_amount: u64,  // Cantidad mínima a quemar
    pub decay_factor: f64, // Factor de decaimiento del poder de minado
    pub burn_address: String,
}

impl ProofOfBurn {
    pub fn new(burn_amount: u64) -> Self {
        ProofOfBurn {
            burn_transactions: Vec::new(),
            burn_amount,
            decay_factor: 0.95, // El poder de minado decrece 5% por bloque
            burn_address: "1111111111111111111114oLvT2".to_string(), // Dirección quemada estándar
        }
    }

    pub fn add_burn_transaction(&mut self, amount: u64, timestamp: i64) -> Result<String, String> {
        if amount < self.burn_amount {
            return Err(format!(
                "Burn amount {} is below minimum {}",
                amount, self.burn_amount
            ));
        }

        // Crear hash de transacción
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}{}", amount, self.burn_address, timestamp));
        let tx_hash = format!("{:x}", hasher.finalize());

        let burn_tx = BurnTransaction {
            amount,
            burn_address: self.burn_address.clone(),
            timestamp,
            tx_hash: tx_hash.clone(),
        };

        self.burn_transactions.push(burn_tx);
        Ok(tx_hash)
    }

    fn calculate_mining_power(&self, current_block_index: u64) -> f64 {
        let mut total_power = 0.0;

        for burn_tx in &self.burn_transactions {
            // Calcular edad de la transacción de quema (en bloques)
            let age = current_block_index.saturating_sub(burn_tx.timestamp as u64);

            // Aplicar factor de decaimiento
            let power = (burn_tx.amount as f64) * self.decay_factor.powf(age as f64);
            total_power += power;
        }

        total_power
    }

    fn select_miner(&self, block: &Block) -> Option<(String, f64)> {
        if self.burn_transactions.is_empty() {
            return None;
        }

        let total_power = self.calculate_mining_power(block.index);
        if total_power == 0.0 {
            return None;
        }

        // Usar el hash del bloque anterior como semilla
        let seed = self.create_seed_from_hash(&block.previous_hash);
        let mut rng = StdRng::from_seed(seed);

        let random_value = rng.random::<f64>() * total_power;
        let mut cumulative_power = 0.0;

        for burn_tx in &self.burn_transactions {
            let age = block.index.saturating_sub(burn_tx.timestamp as u64);
            let power = (burn_tx.amount as f64) * self.decay_factor.powf(age as f64);

            cumulative_power += power;
            if cumulative_power >= random_value {
                return Some((burn_tx.tx_hash.clone(), power));
            }
        }

        // Fallback a la última transacción
        if let Some(last_tx) = self.burn_transactions.last() {
            let age = block.index.saturating_sub(last_tx.timestamp as u64);
            let power = (last_tx.amount as f64) * self.decay_factor.powf(age as f64);
            Some((last_tx.tx_hash.clone(), power))
        } else {
            None
        }
    }

    fn create_seed_from_hash(&self, hash: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(hash.as_bytes());
        hasher.finalize().into()
    }

    fn create_burn_proof(
        &self,
        block: &Block,
        selected_tx_hash: &str,
        mining_power: f64,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}{}{}{}{}{}",
            block.index,
            block.timestamp,
            &block.data,
            &block.previous_hash,
            selected_tx_hash,
            mining_power
        ));
        format!("{:x}", hasher.finalize())
    }

    fn verify_burn_transaction(&self, tx_hash: &str) -> bool {
        self.burn_transactions
            .iter()
            .any(|tx| tx.tx_hash == tx_hash)
    }
}

impl ConsensusAlgorithm for ProofOfBurn {
    fn execute_consensus(&mut self, block: &mut Block) -> Result<ConsensusResult, String> {
        let start_time = Instant::now();

        // Verificar que hay transacciones de quema disponibles
        if self.burn_transactions.is_empty() {
            return Err("No burn transactions available for mining".to_string());
        }

        // Seleccionar minero basado en poder de quema
        let (selected_tx_hash, mining_power) =
            self.select_miner(block).ok_or("Unable to select miner")?;

        // Crear prueba de quema
        let burn_proof = self.create_burn_proof(block, &selected_tx_hash, mining_power);
        block.hash = burn_proof.clone();

        // Usar el hash de la transacción de quema como nonce
        let nonce_hash = &selected_tx_hash[..16]; // Primeros 16 caracteres
        block.nonce = u64::from_str_radix(nonce_hash, 16).unwrap_or(0);

        let duration = start_time.elapsed();

        // Preparar datos de prueba
        let mut proof_data = HashMap::new();
        proof_data.insert("selected_burn_tx".to_string(), selected_tx_hash.clone());
        proof_data.insert("mining_power".to_string(), mining_power.to_string());
        proof_data.insert(
            "total_burns".to_string(),
            self.burn_transactions.len().to_string(),
        );
        proof_data.insert("decay_factor".to_string(), self.decay_factor.to_string());
        proof_data.insert("burn_address".to_string(), self.burn_address.clone());

        // Calcular monedas totales quemadas
        let total_burned: u64 = self.burn_transactions.iter().map(|tx| tx.amount).sum();
        proof_data.insert("total_burned".to_string(), total_burned.to_string());

        Ok(ConsensusResult {
            block: block.clone(),
            proof_data,
            execution_time: duration,
            energy_cost: Some(0.005), // Bajo consumo, principalmente computación
        })
    }

    fn validate_block(&self, block: &Block) -> bool {
        // Verificar que el nonce corresponde a una transacción de quema válida
        let nonce_hex = format!("{:016x}", block.nonce);

        // Buscar transacción de quema que coincida
        for burn_tx in &self.burn_transactions {
            if burn_tx.tx_hash.starts_with(&nonce_hex) {
                // Verificar que la prueba es válida
                let mining_power = self.calculate_mining_power(block.index);
                let expected_proof = self.create_burn_proof(block, &burn_tx.tx_hash, mining_power);
                return expected_proof == block.hash;
            }
        }

        false
    }

    fn get_algorithm_name(&self) -> &'static str {
        "Proof of Burn"
    }

    fn get_energy_efficiency(&self) -> Option<f64> {
        Some(0.90) // Alta eficiencia energética (no requiere minado computacional)
    }

    fn get_statistics(&self) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        stats.insert(
            "total_burn_transactions".to_string(),
            self.burn_transactions.len().to_string(),
        );
        stats.insert(
            "minimum_burn_amount".to_string(),
            self.burn_amount.to_string(),
        );
        stats.insert("decay_factor".to_string(), self.decay_factor.to_string());
        stats.insert("burn_address".to_string(), self.burn_address.clone());

        // Estadísticas de quema
        if !self.burn_transactions.is_empty() {
            let total_burned: u64 = self.burn_transactions.iter().map(|tx| tx.amount).sum();
            let avg_burn = total_burned / self.burn_transactions.len() as u64;
            let max_burn = self
                .burn_transactions
                .iter()
                .map(|tx| tx.amount)
                .max()
                .unwrap_or(0);
            let min_burn = self
                .burn_transactions
                .iter()
                .map(|tx| tx.amount)
                .min()
                .unwrap_or(0);

            stats.insert("total_burned".to_string(), total_burned.to_string());
            stats.insert("average_burn".to_string(), avg_burn.to_string());
            stats.insert("max_burn".to_string(), max_burn.to_string());
            stats.insert("min_burn".to_string(), min_burn.to_string());
        }

        stats
    }

    fn configure(&mut self, config: ConsensusConfig) -> Result<(), String> {
        if let Some(burn_amount_str) = config.additional_params.get("burn_amount") {
            self.burn_amount = burn_amount_str
                .parse()
                .map_err(|_| "Invalid burn_amount parameter".to_string())?;
        }

        if let Some(decay_str) = config.additional_params.get("decay_factor") {
            self.decay_factor = decay_str
                .parse()
                .map_err(|_| "Invalid decay_factor parameter".to_string())?;
        }

        if let Some(burn_addr) = config.additional_params.get("burn_address") {
            self.burn_address = burn_addr.clone();
        }

        Ok(())
    }
}
