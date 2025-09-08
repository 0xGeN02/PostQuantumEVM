use crate::block::Block;
use crate::consensus::{ConsensusAlgorithm, ConsensusFactory, ConsensusResult, ConsensusType};
use crate::logger::BlockchainLogger;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize)]
pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub consensus_type: ConsensusType,
    pub consensus_stats: BlockchainStats,
    pub difficulty: usize,
    #[serde(skip)]
    pub logger: Option<BlockchainLogger>,
    #[serde(skip)]
    consensus_algorithm: Option<Box<dyn ConsensusAlgorithm>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainStats {
    pub total_blocks: u64,
    pub total_mining_time: u64, // in milliseconds
    pub average_block_time: f64,
    pub energy_consumption: f64,
    pub consensus_failures: u64,
}

impl Default for BlockchainStats {
    fn default() -> Self {
        BlockchainStats {
            total_blocks: 0,
            total_mining_time: 0,
            average_block_time: 0.0,
            energy_consumption: 0.0,
            consensus_failures: 0,
        }
    }
}

impl Blockchain {
    /// Crea una nueva blockchain con el consenso especificado
    pub fn new_with_consensus(consensus_type: ConsensusType) -> Result<Self, String> {
        let logger = BlockchainLogger::new();
        let consensus_algorithm = ConsensusFactory::create_consensus(&consensus_type)?;

        let mut genesis_block = Block::new(0, "Genesis Block".to_string(), "0".to_string());

        // Crear copia del algoritmo de consenso para el bloque génesis
        let mut genesis_consensus = ConsensusFactory::create_consensus(&consensus_type)?;

        // Ejecutar consenso para el bloque génesis
        match genesis_consensus.execute_consensus(&mut genesis_block) {
            Ok(result) => {
                genesis_block.set_consensus_data(result.proof_data);

                // Log de creación del bloque génesis
                if let Some(ref logger) = Some(&logger) {
                    logger.log_block_creation(&genesis_block);
                }
            }
            Err(e) => {
                println!("Warning: Genesis block consensus failed: {}", e);
                // Para el bloque génesis, continuamos aunque falle el consenso
            }
        }

        let mut blockchain = Blockchain {
            blocks: vec![genesis_block],
            consensus_type,
            consensus_stats: BlockchainStats::default(),
            difficulty: 1, // Default difficulty
            logger: Some(logger),
            consensus_algorithm: Some(consensus_algorithm),
        };

        blockchain.consensus_stats.total_blocks = 1;
        Ok(blockchain)
    }

    /// Constructor legacy que usa PoW por defecto
    pub fn new() -> Self {
        Self::new_with_consensus(ConsensusType::default())
            .expect("Default consensus should always work")
    }

    /// Cambia el algoritmo de consenso
    pub fn switch_consensus(&mut self, new_consensus_type: ConsensusType) -> Result<(), String> {
        println!(
            "Switching consensus from {} to {}",
            self.consensus_type.name(),
            new_consensus_type.name()
        );

        let new_algorithm = ConsensusFactory::create_consensus(&new_consensus_type)?;
        self.consensus_type = new_consensus_type;
        self.consensus_algorithm = Some(new_algorithm);

        println!("✅ Consensus algorithm switched successfully");
        Ok(())
    }

    /// Añade un nuevo bloque usando el algoritmo de consenso configurado
    pub fn add_block(&mut self, data: String) -> Result<ConsensusResult, String> {
        let previous_hash = self
            .blocks
            .last()
            .ok_or("No blocks in blockchain")?
            .hash
            .clone();

        let mut new_block = Block::new(self.blocks.len() as u64, data, previous_hash);

        // Obtener el algoritmo de consenso actual
        let consensus_algorithm = self
            .consensus_algorithm
            .as_mut()
            .ok_or("No consensus algorithm configured")?;

        // Log de inicio de minado
        if let Some(ref logger) = self.logger {
            logger.log_mining_start(new_block.index, new_block.difficulty);
        }

        let start = std::time::Instant::now();

        // Ejecutar consenso
        match consensus_algorithm.execute_consensus(&mut new_block) {
            Ok(result) => {
                let duration = start.elapsed();

                // Actualizar estadísticas
                self.update_stats(&result, duration);

                // Actualizar datos del bloque
                new_block.set_consensus_data(result.proof_data.clone());

                // Log de finalización
                if let Some(ref logger) = self.logger {
                    logger.log_mining_complete(&new_block, duration);
                    logger.log_block_creation(&new_block);
                }

                self.blocks.push(new_block);
                Ok(result)
            }
            Err(e) => {
                self.consensus_stats.consensus_failures += 1;
                Err(format!("Consensus failed: {}", e))
            }
        }
    }

    /// Valida toda la blockchain usando el algoritmo de consenso actual
    pub fn is_valid(&self) -> bool {
        let consensus_algorithm = match &self.consensus_algorithm {
            Some(algo) => algo,
            None => return false,
        };

        for i in 1..self.blocks.len() {
            let current = &self.blocks[i];
            let previous = &self.blocks[i - 1];

            // Verificar enlaces entre bloques
            if current.previous_hash != previous.hash {
                println!(
                    "❌ Invalid block chain at block {}: previous hash mismatch",
                    i
                );
                return false;
            }

            // Validar usando el algoritmo de consenso
            if !consensus_algorithm.validate_block(current) {
                println!("❌ Invalid consensus proof at block {}", i);
                return false;
            }
        }

        true
    }

    /// Actualiza las estadísticas de la blockchain
    fn update_stats(&mut self, result: &ConsensusResult, duration: Duration) {
        self.consensus_stats.total_blocks += 1;
        self.consensus_stats.total_mining_time += duration.as_millis() as u64;

        // Calcular tiempo promedio de bloque
        self.consensus_stats.average_block_time = self.consensus_stats.total_mining_time as f64
            / self.consensus_stats.total_blocks as f64;

        // Sumar consumo energético si está disponible
        if let Some(energy) = result.energy_cost {
            self.consensus_stats.energy_consumption += energy;
        }
    }

    /// Obtiene información detallada del algoritmo de consenso actual
    pub fn get_consensus_info(&self) -> Result<std::collections::HashMap<String, String>, String> {
        let consensus_algorithm = self
            .consensus_algorithm
            .as_ref()
            .ok_or("No consensus algorithm configured")?;

        let mut info = consensus_algorithm.get_statistics();
        info.insert(
            "algorithm_name".to_string(),
            consensus_algorithm.get_algorithm_name().to_string(),
        );
        info.insert(
            "total_blocks".to_string(),
            self.consensus_stats.total_blocks.to_string(),
        );
        info.insert(
            "average_block_time_ms".to_string(),
            self.consensus_stats.average_block_time.to_string(),
        );
        info.insert(
            "total_energy_consumption".to_string(),
            self.consensus_stats.energy_consumption.to_string(),
        );
        info.insert(
            "consensus_failures".to_string(),
            self.consensus_stats.consensus_failures.to_string(),
        );

        if let Some(efficiency) = consensus_algorithm.get_energy_efficiency() {
            info.insert("energy_efficiency".to_string(), efficiency.to_string());
        }

        Ok(info)
    }

    /// Método legacy para compatibilidad
    #[deprecated(note = "Use calculate_adaptive_difficulty instead")]
    fn calculate_next_difficulty(&self) -> usize {
        if self.blocks.len() % 2 == 0 && self.blocks.len() > 0 {
            let last_difficulty = self.blocks.last().unwrap().difficulty;
            std::cmp::min(last_difficulty + 1, 6)
        } else {
            4 // Default difficulty
        }
    }

    /// Calcula dificultad adaptativa usando el algoritmo de consenso
    pub fn calculate_adaptive_difficulty(&self) -> Option<usize> {
        let consensus_algorithm = self.consensus_algorithm.as_ref()?;
        consensus_algorithm.calculate_next_difficulty(&self.blocks)
    }

    /// Método legacy mantenido para compatibilidad
    pub fn get_difficulty_stats(&self) -> (usize, usize, f64) {
        let difficulties: Vec<usize> = self.blocks.iter().map(|b| b.difficulty).collect();
        let min_diff = *difficulties.iter().min().unwrap_or(&0);
        let max_diff = *difficulties.iter().max().unwrap_or(&0);
        let avg_diff = difficulties.iter().sum::<usize>() as f64 / difficulties.len() as f64;
        (min_diff, max_diff, avg_diff)
    }

    /// Obtiene estadísticas detalladas de la blockchain
    pub fn get_blockchain_stats(&self) -> &BlockchainStats {
        &self.consensus_stats
    }

    /// Obtiene estadísticas de consenso específicas
    pub fn get_consensus_statistics(&self) -> std::collections::HashMap<String, String> {
        match self.get_consensus_info() {
            Ok(info) => info,
            Err(_) => std::collections::HashMap::new(),
        }
    }

    /// Compara eficiencia entre diferentes algoritmos de consenso
    pub fn benchmark_consensus_algorithms(
        &self,
        test_data: Vec<String>,
    ) -> Result<Vec<(String, Duration, f64)>, String> {
        let algorithms = vec![
            ConsensusType::ProofOfWork { difficulty: 2 },
            ConsensusType::ProofOfStake {
                minimum_stake: 1000,
            },
            ConsensusType::ProofOfAuthority {
                validators: vec!["validator1".to_string(), "validator2".to_string()],
            },
            ConsensusType::ProofOfHistory {
                vdf_iterations: 1000,
            },
        ];

        let mut results = Vec::new();

        for consensus_type in algorithms {
            let mut test_algorithm = ConsensusFactory::create_consensus(&consensus_type)?;
            let algorithm_name = test_algorithm.get_algorithm_name().to_string();

            let mut total_time = Duration::new(0, 0);
            let mut total_energy = 0.0;

            for (i, data) in test_data.iter().enumerate() {
                let mut test_block = Block::new(i as u64, data.clone(), "test_hash".to_string());
                let start = std::time::Instant::now();

                match test_algorithm.execute_consensus(&mut test_block) {
                    Ok(result) => {
                        total_time += start.elapsed();
                        if let Some(energy) = result.energy_cost {
                            total_energy += energy;
                        }
                    }
                    Err(_) => continue, // Skip failed consensus
                }
            }

            results.push((algorithm_name, total_time, total_energy));
        }

        Ok(results)
    }

    // === Métodos de logging (mantenidos para compatibilidad) ===

    pub fn log_blockchain_state(&self) {
        if let Some(ref logger) = self.logger {
            logger.log_blockchain_state(self);
        }
    }

    pub fn log_validation_result(&self) -> bool {
        let is_valid = self.is_valid();
        if let Some(ref logger) = self.logger {
            logger.log_validation_result(is_valid);
        }
        is_valid
    }

    pub fn log_difficulty_stats(&self) {
        let (min_diff, max_diff, avg_diff) = self.get_difficulty_stats();
        if let Some(ref logger) = self.logger {
            logger.log_difficulty_stats(min_diff, max_diff, avg_diff);
        }
    }

    pub fn create_summary_report(&self) {
        if let Some(ref logger) = self.logger {
            logger.create_summary_report(self);
        }
    }

    /// Nuevo método para log de estadísticas de consenso
    pub fn log_consensus_statistics(&self) {
        if let Ok(stats) = self.get_consensus_info() {
            println!("\n=== Consensus Statistics ===");
            println!(
                "Algorithm: {}",
                stats
                    .get("algorithm_name")
                    .unwrap_or(&"Unknown".to_string())
            );
            println!(
                "Total Blocks: {}",
                stats.get("total_blocks").unwrap_or(&"0".to_string())
            );
            println!(
                "Average Block Time: {:.2} ms",
                stats
                    .get("average_block_time_ms")
                    .unwrap_or(&"0".to_string())
                    .parse::<f64>()
                    .unwrap_or(0.0)
            );
            println!(
                "Energy Consumption: {:.6}",
                stats
                    .get("total_energy_consumption")
                    .unwrap_or(&"0".to_string())
                    .parse::<f64>()
                    .unwrap_or(0.0)
            );

            if let Some(efficiency) = stats.get("energy_efficiency") {
                println!(
                    "Energy Efficiency: {:.2}%",
                    efficiency.parse::<f64>().unwrap_or(0.0) * 100.0
                );
            }

            println!(
                "Consensus Failures: {}",
                stats.get("consensus_failures").unwrap_or(&"0".to_string())
            );
            println!("=============================\n");
        }
    }
}
