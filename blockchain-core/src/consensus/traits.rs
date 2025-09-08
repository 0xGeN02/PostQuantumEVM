use crate::block::Block;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Resultado del proceso de minado/consenso
#[derive(Debug, Clone)]
pub struct ConsensusResult {
    pub block: Block,
    pub proof_data: HashMap<String, String>,
    pub execution_time: Duration,
    pub energy_cost: Option<f64>,
}

/// Configuración adicional para diferentes algoritmos de consenso
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    pub additional_params: HashMap<String, String>,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        ConsensusConfig {
            additional_params: HashMap::new(),
        }
    }
}

/// Trait principal para todos los algoritmos de consenso
pub trait ConsensusAlgorithm: Send + Sync {
    /// Ejecuta el algoritmo de consenso para un bloque
    fn execute_consensus(&mut self, block: &mut Block) -> Result<ConsensusResult, String>;

    /// Valida si un bloque cumple con el algoritmo de consenso
    fn validate_block(&self, block: &Block) -> bool;

    /// Devuelve el nombre del algoritmo de consenso
    fn get_algorithm_name(&self) -> &'static str;

    /// Calcula la dificultad siguiente (si aplica)
    fn calculate_next_difficulty(&self, _blocks: &[Block]) -> Option<usize> {
        None // Por defecto, no todos los algoritmos usan dificultad
    }

    /// Devuelve el costo energético estimado (si aplica)
    fn get_energy_efficiency(&self) -> Option<f64> {
        None
    }

    /// Devuelve estadísticas específicas del algoritmo
    fn get_statistics(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    /// Configura parámetros específicos del algoritmo
    fn configure(&mut self, _config: ConsensusConfig) -> Result<(), String> {
        Ok(())
    }
}
