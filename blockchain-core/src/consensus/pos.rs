use crate::block::Block;
use crate::consensus::traits::{ConsensusAlgorithm, ConsensusConfig, ConsensusResult};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Validator {
    pub address: String,
    pub stake: u64,
    pub reputation: f64,
}

#[derive(Debug, Clone)]
pub struct ProofOfStake {
    pub validators: Vec<Validator>,
    pub minimum_stake: u64,
    pub slashing_rate: f64, // Porcentaje de stake perdido por mal comportamiento
}

impl ProofOfStake {
    pub fn new(minimum_stake: u64) -> Self {
        ProofOfStake {
            validators: Vec::new(),
            minimum_stake,
            slashing_rate: 0.1, // 10% por defecto
        }
    }

    pub fn add_validator(&mut self, address: String, stake: u64) -> Result<(), String> {
        if stake < self.minimum_stake {
            return Err(format!(
                "Stake {} is below minimum {}",
                stake, self.minimum_stake
            ));
        }

        let validator = Validator {
            address,
            stake,
            reputation: 1.0,
        };

        self.validators.push(validator);
        Ok(())
    }

    fn select_validator(&self, block: &Block) -> Option<&Validator> {
        if self.validators.is_empty() {
            return None;
        }

        // Usar el hash del bloque anterior como semilla para determinismo
        let seed = self.create_seed_from_hash(&block.previous_hash);
        let mut rng = StdRng::from_seed(seed);

        // Calcular stake total ponderado por reputación
        let total_weighted_stake: f64 = self
            .validators
            .iter()
            .map(|v| v.stake as f64 * v.reputation)
            .sum();

        if total_weighted_stake == 0.0 {
            return None;
        }

        let random_value = rng.random::<f64>() * total_weighted_stake;
        let mut cumulative_stake = 0.0;

        for validator in &self.validators {
            cumulative_stake += validator.stake as f64 * validator.reputation;
            if cumulative_stake >= random_value {
                return Some(validator);
            }
        }

        // Fallback al último validador
        self.validators.last()
    }

    fn create_seed_from_hash(&self, hash: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(hash.as_bytes());
        hasher.finalize().into()
    }

    fn create_block_signature(&self, block: &Block, validator: &Validator) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}{}{}{}{}{}",
            block.index,
            block.timestamp,
            &block.data,
            &block.previous_hash,
            &validator.address,
            validator.stake
        ));
        format!("{:x}", hasher.finalize())
    }

    fn calculate_rewards(&self, validator: &Validator, _block: &Block) -> u64 {
        // Recompensa básica proporcional al stake
        let base_reward = (validator.stake / 1000).max(1);
        (base_reward as f64 * validator.reputation) as u64
    }
}

impl ConsensusAlgorithm for ProofOfStake {
    fn execute_consensus(&mut self, block: &mut Block) -> Result<ConsensusResult, String> {
        let start_time = Instant::now();

        // Seleccionar validador
        let validator = self
            .select_validator(block)
            .ok_or("No validators available")?;

        // Crear firma del bloque
        let signature = self.create_block_signature(block, validator);
        block.hash = signature.clone();

        // En PoS no hay nonce tradicional, pero usamos stake como identificador
        block.nonce = validator.stake;

        let duration = start_time.elapsed();

        // Preparar datos de prueba
        let mut proof_data = HashMap::new();
        proof_data.insert("validator_address".to_string(), validator.address.clone());
        proof_data.insert("validator_stake".to_string(), validator.stake.to_string());
        proof_data.insert(
            "validator_reputation".to_string(),
            validator.reputation.to_string(),
        );
        proof_data.insert("signature".to_string(), signature);
        proof_data.insert(
            "reward".to_string(),
            self.calculate_rewards(validator, block).to_string(),
        );

        Ok(ConsensusResult {
            block: block.clone(),
            proof_data,
            execution_time: duration,
            energy_cost: Some(0.001), // Muy bajo consumo energético
        })
    }

    fn validate_block(&self, block: &Block) -> bool {
        // Buscar el validador que supuestamente creó este bloque
        let validator = self.validators.iter().find(|v| v.stake == block.nonce);

        match validator {
            Some(v) => {
                let expected_signature = self.create_block_signature(block, v);
                expected_signature == block.hash
            }
            None => false,
        }
    }

    fn get_algorithm_name(&self) -> &'static str {
        "Proof of Stake"
    }

    fn get_energy_efficiency(&self) -> Option<f64> {
        Some(0.99) // Muy alta eficiencia energética
    }

    fn get_statistics(&self) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        stats.insert(
            "validator_count".to_string(),
            self.validators.len().to_string(),
        );
        stats.insert("minimum_stake".to_string(), self.minimum_stake.to_string());
        stats.insert(
            "total_stake".to_string(),
            self.validators
                .iter()
                .map(|v| v.stake)
                .sum::<u64>()
                .to_string(),
        );
        stats.insert(
            "average_reputation".to_string(),
            (self.validators.iter().map(|v| v.reputation).sum::<f64>()
                / self.validators.len() as f64)
                .to_string(),
        );
        stats.insert("slashing_rate".to_string(), self.slashing_rate.to_string());
        stats
    }

    fn configure(&mut self, config: ConsensusConfig) -> Result<(), String> {
        if let Some(min_stake_str) = config.additional_params.get("minimum_stake") {
            self.minimum_stake = min_stake_str
                .parse()
                .map_err(|_| "Invalid minimum_stake parameter".to_string())?;
        }

        if let Some(slashing_str) = config.additional_params.get("slashing_rate") {
            self.slashing_rate = slashing_str
                .parse()
                .map_err(|_| "Invalid slashing_rate parameter".to_string())?;
        }

        Ok(())
    }
}
