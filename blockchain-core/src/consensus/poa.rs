use crate::block::Block;
use crate::consensus::traits::{ConsensusAlgorithm, ConsensusConfig, ConsensusResult};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Authority {
    pub address: String,
    pub public_key: String,
    pub reputation_score: u64,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct ProofOfAuthority {
    pub authorities: Vec<Authority>,
    pub current_authority_index: usize,
    pub block_interval: Duration, // Tiempo entre bloques
    pub required_confirmations: usize,
}

impl ProofOfAuthority {
    pub fn new(authorities: Vec<String>) -> Self {
        let auth_list: Vec<Authority> = authorities
            .into_iter()
            .enumerate()
            .map(|(i, addr)| Authority {
                address: addr,
                public_key: format!("pubkey_{}", i), // Simplificado
                reputation_score: 100,
                is_active: true,
            })
            .collect();

        ProofOfAuthority {
            authorities: auth_list,
            current_authority_index: 0,
            block_interval: Duration::from_secs(15), // 15 segundos entre bloques
            required_confirmations: 2,
        }
    }

    pub fn add_authority(&mut self, address: String, public_key: String) -> Result<(), String> {
        // En implementación real, esto requeriría consenso de autoridades existentes
        if self.authorities.iter().any(|a| a.address == address) {
            return Err("Authority already exists".to_string());
        }

        let authority = Authority {
            address,
            public_key,
            reputation_score: 100,
            is_active: true,
        };

        self.authorities.push(authority);
        Ok(())
    }

    pub fn remove_authority(&mut self, address: &str) -> Result<(), String> {
        let pos = self
            .authorities
            .iter()
            .position(|a| a.address == address)
            .ok_or("Authority not found")?;

        if self.authorities.len() <= 1 {
            return Err("Cannot remove last authority".to_string());
        }

        self.authorities.remove(pos);

        // Ajustar índice si es necesario
        if self.current_authority_index >= self.authorities.len() {
            self.current_authority_index = 0;
        }

        Ok(())
    }

    fn get_current_authority(&self) -> Option<&Authority> {
        self.authorities
            .get(self.current_authority_index)
            .filter(|a| a.is_active)
    }

    fn rotate_authority(&mut self) {
        self.current_authority_index = (self.current_authority_index + 1) % self.authorities.len();

        // Buscar próxima autoridad activa
        let start_index = self.current_authority_index;
        loop {
            if let Some(authority) = self.authorities.get(self.current_authority_index) {
                if authority.is_active {
                    break;
                }
            }

            self.current_authority_index =
                (self.current_authority_index + 1) % self.authorities.len();

            // Evitar loop infinito
            if self.current_authority_index == start_index {
                break;
            }
        }
    }

    fn create_authority_signature(&self, block: &Block, authority: &Authority) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}{}{}{}{}{}",
            block.index,
            block.timestamp,
            &block.data,
            &block.previous_hash,
            &authority.address,
            &authority.public_key
        ));
        format!("{:x}", hasher.finalize())
    }

    fn validate_authority_signature(&self, block: &Block, signature: &str) -> bool {
        for authority in &self.authorities {
            if authority.is_active {
                let expected_signature = self.create_authority_signature(block, authority);
                if expected_signature == signature {
                    return true;
                }
            }
        }
        false
    }
}

impl ConsensusAlgorithm for ProofOfAuthority {
    fn execute_consensus(&mut self, block: &mut Block) -> Result<ConsensusResult, String> {
        let start_time = Instant::now();

        // Verificar que hay autoridades disponibles
        let authority = self
            .get_current_authority()
            .ok_or("No active authorities available")?
            .clone();

        // Crear firma de autoridad
        let signature = self.create_authority_signature(block, &authority);
        block.hash = signature.clone();

        // El nonce contiene el índice de la autoridad
        block.nonce = self.current_authority_index as u64;

        // Rotar a la siguiente autoridad
        self.rotate_authority();

        let duration = start_time.elapsed();

        // Preparar datos de prueba
        let mut proof_data = HashMap::new();
        proof_data.insert("authority_address".to_string(), authority.address.clone());
        proof_data.insert(
            "authority_index".to_string(),
            (block.nonce as usize).to_string(),
        );
        proof_data.insert(
            "authority_reputation".to_string(),
            authority.reputation_score.to_string(),
        );
        proof_data.insert("signature".to_string(), signature);
        proof_data.insert(
            "block_interval_seconds".to_string(),
            self.block_interval.as_secs().to_string(),
        );

        Ok(ConsensusResult {
            block: block.clone(),
            proof_data,
            execution_time: duration,
            energy_cost: Some(0.0001), // Muy bajo consumo
        })
    }

    fn validate_block(&self, block: &Block) -> bool {
        // Verificar que el índice de autoridad es válido
        let authority_index = block.nonce as usize;
        if authority_index >= self.authorities.len() {
            return false;
        }

        // Verificar que la autoridad estaba activa
        if let Some(authority) = self.authorities.get(authority_index) {
            if !authority.is_active {
                return false;
            }

            // Verificar la firma
            let expected_signature = self.create_authority_signature(block, authority);
            return expected_signature == block.hash;
        }

        false
    }

    fn get_algorithm_name(&self) -> &'static str {
        "Proof of Authority"
    }

    fn get_energy_efficiency(&self) -> Option<f64> {
        Some(0.995) // Muy alta eficiencia energética
    }

    fn get_statistics(&self) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        stats.insert(
            "total_authorities".to_string(),
            self.authorities.len().to_string(),
        );
        stats.insert(
            "active_authorities".to_string(),
            self.authorities
                .iter()
                .filter(|a| a.is_active)
                .count()
                .to_string(),
        );
        stats.insert(
            "current_authority_index".to_string(),
            self.current_authority_index.to_string(),
        );
        stats.insert(
            "block_interval_seconds".to_string(),
            self.block_interval.as_secs().to_string(),
        );
        stats.insert(
            "required_confirmations".to_string(),
            self.required_confirmations.to_string(),
        );

        if let Some(current_auth) = self.get_current_authority() {
            stats.insert(
                "current_authority".to_string(),
                current_auth.address.clone(),
            );
            stats.insert(
                "current_authority_reputation".to_string(),
                current_auth.reputation_score.to_string(),
            );
        }

        stats
    }

    fn configure(&mut self, config: ConsensusConfig) -> Result<(), String> {
        if let Some(interval_str) = config.additional_params.get("block_interval_seconds") {
            let seconds: u64 = interval_str
                .parse()
                .map_err(|_| "Invalid block_interval_seconds parameter".to_string())?;
            self.block_interval = Duration::from_secs(seconds);
        }

        if let Some(confirmations_str) = config.additional_params.get("required_confirmations") {
            self.required_confirmations = confirmations_str
                .parse()
                .map_err(|_| "Invalid required_confirmations parameter".to_string())?;
        }

        Ok(())
    }
}
