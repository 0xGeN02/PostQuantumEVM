use crate::block::Block;
use crate::consensus::traits::{ConsensusAlgorithm, ConsensusConfig, ConsensusResult};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    PrePrepare,
    Prepare,
    Commit,
}

#[derive(Debug, Clone)]
pub struct PbftMessage {
    pub message_type: MessageType,
    pub view: u64,
    pub sequence: u64,
    pub block_hash: String,
    pub node_id: String,
    pub signature: String,
}

#[derive(Debug, Clone)]
pub struct PbftNode {
    pub node_id: String,
    pub is_primary: bool,
    pub is_faulty: bool,
    pub reputation: f64,
}

#[derive(Debug, Clone)]
pub struct PracticalByzantineFaultTolerance {
    pub nodes: Vec<PbftNode>,
    pub node_count: usize,
    pub fault_tolerance: f32, // Porcentaje de nodos maliciosos tolerados (típicamente 33%)
    pub current_view: u64,
    pub current_sequence: u64,
    pub message_log: Vec<PbftMessage>,
}

impl PracticalByzantineFaultTolerance {
    pub fn new(node_count: usize, fault_tolerance: f32) -> Self {
        if fault_tolerance >= 0.33 {
            println!("Warning: pBFT typically tolerates up to 33% faulty nodes");
        }

        let nodes: Vec<PbftNode> = (0..node_count)
            .map(|i| PbftNode {
                node_id: format!("node_{}", i),
                is_primary: i == 0, // El primer nodo es primario inicial
                is_faulty: false,
                reputation: 1.0,
            })
            .collect();

        PracticalByzantineFaultTolerance {
            nodes,
            node_count,
            fault_tolerance,
            current_view: 0,
            current_sequence: 0,
            message_log: Vec::new(),
        }
    }

    pub fn set_faulty_nodes(&mut self, faulty_node_indices: Vec<usize>) -> Result<(), String> {
        let max_faulty = (self.node_count as f32 * self.fault_tolerance).floor() as usize;

        if faulty_node_indices.len() > max_faulty {
            return Err(format!(
                "Too many faulty nodes: {} > {}",
                faulty_node_indices.len(),
                max_faulty
            ));
        }

        // Resetear todos los nodos a no maliciosos
        for node in &mut self.nodes {
            node.is_faulty = false;
        }

        // Marcar nodos maliciosos
        for &index in &faulty_node_indices {
            if index < self.node_count {
                self.nodes[index].is_faulty = true;
                self.nodes[index].reputation = 0.1; // Baja reputación
            }
        }

        Ok(())
    }

    fn get_primary_node(&self) -> Option<&PbftNode> {
        self.nodes.iter().find(|n| n.is_primary && !n.is_faulty)
    }

    fn get_honest_nodes(&self) -> Vec<&PbftNode> {
        self.nodes.iter().filter(|n| !n.is_faulty).collect()
    }

    fn get_honest_nodes_owned(&self) -> Vec<PbftNode> {
        self.nodes
            .iter()
            .filter(|n| !n.is_faulty)
            .cloned()
            .collect()
    }

    fn create_message(&self, msg_type: MessageType, block: &Block, node_id: &str) -> PbftMessage {
        let signature = self.sign_message(&msg_type, block, node_id);

        PbftMessage {
            message_type: msg_type,
            view: self.current_view,
            sequence: self.current_sequence,
            block_hash: block.hash.clone(),
            node_id: node_id.to_string(),
            signature,
        }
    }

    fn sign_message(&self, msg_type: &MessageType, block: &Block, node_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{:?}{}{}{}{}{}",
            msg_type,
            self.current_view,
            self.current_sequence,
            &block.hash,
            node_id,
            block.timestamp
        ));
        format!("{:x}", hasher.finalize())
    }

    fn verify_message(&self, message: &PbftMessage, block: &Block) -> bool {
        let expected_signature = self.sign_message(&message.message_type, block, &message.node_id);
        expected_signature == message.signature
            && message.view == self.current_view
            && message.sequence == self.current_sequence
    }

    /// Simula el proceso de consenso pBFT en tres fases
    fn execute_pbft_consensus(&mut self, block: &Block) -> Result<bool, String> {
        let honest_nodes = self.get_honest_nodes_owned(); // Get owned nodes to avoid borrowing issues
        let required_votes = (honest_nodes.len() * 2 / 3) + 1; // 2f + 1 votos necesarios

        // Fase 1: Pre-prepare (solo del nodo primario)
        let primary = self
            .get_primary_node()
            .ok_or("No primary node available")?
            .clone();

        let pre_prepare_msg = self.create_message(MessageType::PrePrepare, block, &primary.node_id);
        self.message_log.push(pre_prepare_msg);

        // Fase 2: Prepare (todos los nodos honestos)
        let mut prepare_votes = 0;
        for node in &honest_nodes {
            if !node.is_primary {
                let prepare_msg = self.create_message(MessageType::Prepare, block, &node.node_id);
                if self.verify_message(&prepare_msg, block) {
                    prepare_votes += 1;
                    self.message_log.push(prepare_msg);
                }
            }
        }

        if prepare_votes < required_votes - 1 {
            // -1 porque el primario no vota en prepare
            return Ok(false);
        }

        // Fase 3: Commit (todos los nodos honestos)
        let mut commit_votes = 0;
        for node in &honest_nodes {
            let commit_msg = self.create_message(MessageType::Commit, block, &node.node_id);
            if self.verify_message(&commit_msg, block) {
                commit_votes += 1;
                self.message_log.push(commit_msg);
            }
        }

        // Verificar si tenemos suficientes votos para consenso
        Ok(commit_votes >= required_votes)
    }

    fn rotate_primary(&mut self) {
        // Rotar al siguiente nodo honesto
        let current_primary_idx = self.nodes.iter().position(|n| n.is_primary).unwrap_or(0);

        // Desmarcar primario actual
        if let Some(current_primary) = self.nodes.get_mut(current_primary_idx) {
            current_primary.is_primary = false;
        }

        // Buscar siguiente nodo honesto
        for i in 1..=self.node_count {
            let next_idx = (current_primary_idx + i) % self.node_count;
            if let Some(next_node) = self.nodes.get_mut(next_idx) {
                if !next_node.is_faulty {
                    next_node.is_primary = true;
                    break;
                }
            }
        }

        self.current_view += 1;
    }

    fn calculate_consensus_metrics(&self) -> (usize, usize, f32) {
        let honest_count = self.nodes.iter().filter(|n| !n.is_faulty).count();
        let faulty_count = self.nodes.iter().filter(|n| n.is_faulty).count();
        let fault_percentage = (faulty_count as f32 / self.node_count as f32) * 100.0;

        (honest_count, faulty_count, fault_percentage)
    }
}

impl ConsensusAlgorithm for PracticalByzantineFaultTolerance {
    fn execute_consensus(&mut self, block: &mut Block) -> Result<ConsensusResult, String> {
        let start_time = Instant::now();

        if self.nodes.is_empty() {
            return Err("No nodes available for pBFT consensus".to_string());
        }

        // Verificar que tenemos suficientes nodos honestos
        let honest_nodes = self.get_honest_nodes_owned();
        let required_honest = (self.node_count * 2 / 3) + 1;

        if honest_nodes.len() < required_honest {
            return Err(format!(
                "Insufficient honest nodes: {} < {}",
                honest_nodes.len(),
                required_honest
            ));
        }

        // Ejecutar consenso pBFT
        let consensus_reached = self.execute_pbft_consensus(block)?;

        if !consensus_reached {
            return Err("pBFT consensus not reached".to_string());
        }

        // Crear hash final del bloque con información de consenso
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}{}{}{}{}{}{}",
            block.index,
            block.timestamp,
            &block.data,
            &block.previous_hash,
            self.current_view,
            self.current_sequence,
            honest_nodes.len()
        ));

        block.hash = format!("{:x}", hasher.finalize());
        block.nonce = self.current_sequence; // Usar número de secuencia como nonce

        // Incrementar secuencia para el siguiente bloque
        self.current_sequence += 1;

        // Rotar primario ocasionalmente
        if self.current_sequence % 10 == 0 {
            self.rotate_primary();
        }

        let duration = start_time.elapsed();
        let (honest_count, faulty_count, fault_percentage) = self.calculate_consensus_metrics();

        // Preparar datos de prueba
        let mut proof_data = HashMap::new();
        proof_data.insert("consensus_view".to_string(), self.current_view.to_string());
        proof_data.insert(
            "sequence_number".to_string(),
            (self.current_sequence - 1).to_string(),
        );
        proof_data.insert("total_nodes".to_string(), self.node_count.to_string());
        proof_data.insert("honest_nodes".to_string(), honest_count.to_string());
        proof_data.insert("faulty_nodes".to_string(), faulty_count.to_string());
        proof_data.insert("fault_percentage".to_string(), fault_percentage.to_string());
        proof_data.insert(
            "messages_processed".to_string(),
            self.message_log.len().to_string(),
        );

        if let Some(primary) = self.get_primary_node() {
            proof_data.insert("primary_node".to_string(), primary.node_id.clone());
        }

        Ok(ConsensusResult {
            block: block.clone(),
            proof_data,
            execution_time: duration,
            energy_cost: Some(0.02), // Moderado consumo (comunicación entre nodos)
        })
    }

    fn validate_block(&self, block: &Block) -> bool {
        // Verificación básica: el nonce debe corresponder a una secuencia válida
        if block.nonce == 0 && self.current_sequence > 1 {
            return false;
        }

        // Verificar que el hash incluye información de consenso válida
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}{}{}{}{}{}{}",
            block.index,
            block.timestamp,
            &block.data,
            &block.previous_hash,
            self.current_view,
            block.nonce, // secuencia
            self.get_honest_nodes().len()
        ));

        let expected_hash = format!("{:x}", hasher.finalize());
        expected_hash == block.hash
    }

    fn get_algorithm_name(&self) -> &'static str {
        "Practical Byzantine Fault Tolerance"
    }

    fn get_energy_efficiency(&self) -> Option<f64> {
        Some(0.80) // Moderada eficiencia (requiere comunicación intensiva)
    }

    fn get_statistics(&self) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        let (honest_count, faulty_count, fault_percentage) = self.calculate_consensus_metrics();

        stats.insert("total_nodes".to_string(), self.node_count.to_string());
        stats.insert("honest_nodes".to_string(), honest_count.to_string());
        stats.insert("faulty_nodes".to_string(), faulty_count.to_string());
        stats.insert(
            "fault_tolerance_percent".to_string(),
            (self.fault_tolerance * 100.0).to_string(),
        );
        stats.insert(
            "current_fault_percent".to_string(),
            fault_percentage.to_string(),
        );
        stats.insert("current_view".to_string(), self.current_view.to_string());
        stats.insert(
            "current_sequence".to_string(),
            self.current_sequence.to_string(),
        );
        stats.insert(
            "total_messages".to_string(),
            self.message_log.len().to_string(),
        );

        // Estadísticas del primario
        if let Some(primary) = self.get_primary_node() {
            stats.insert("primary_node".to_string(), primary.node_id.clone());
            stats.insert(
                "primary_reputation".to_string(),
                primary.reputation.to_string(),
            );
        }

        // Calcular reputación promedio
        let avg_reputation =
            self.nodes.iter().map(|n| n.reputation).sum::<f64>() / self.node_count as f64;
        stats.insert("average_reputation".to_string(), avg_reputation.to_string());

        stats
    }

    fn configure(&mut self, config: ConsensusConfig) -> Result<(), String> {
        if let Some(node_count_str) = config.additional_params.get("node_count") {
            let new_node_count: usize = node_count_str
                .parse()
                .map_err(|_| "Invalid node_count parameter".to_string())?;

            // Recrear nodos si cambia el conteo
            if new_node_count != self.node_count {
                self.nodes = (0..new_node_count)
                    .map(|i| PbftNode {
                        node_id: format!("node_{}", i),
                        is_primary: i == 0,
                        is_faulty: false,
                        reputation: 1.0,
                    })
                    .collect();
                self.node_count = new_node_count;
            }
        }

        if let Some(fault_tolerance_str) = config.additional_params.get("fault_tolerance") {
            self.fault_tolerance = fault_tolerance_str
                .parse()
                .map_err(|_| "Invalid fault_tolerance parameter".to_string())?;
        }

        Ok(())
    }
}
