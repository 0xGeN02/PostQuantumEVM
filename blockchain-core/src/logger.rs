use crate::block::Block;
use crate::blockchain::Blockchain;
use chrono::Utc;
use serde_json;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct BlockchainLogger {
    logs_dir: String,
    session_id: String,
}

impl BlockchainLogger {
    pub fn new() -> Self {
        let timestamp = Utc::now();
        let session_id = timestamp.format("%Y%m%d_%H%M%S").to_string();

        // Obtener el directorio del ejecutable y navegar a blockchain-core
        let current_exe = std::env::current_exe().unwrap();
        let exe_dir = current_exe.parent().unwrap();

        // Si estamos en target/debug, subimos dos niveles y entramos a blockchain-core
        let blockchain_core_dir = if exe_dir.ends_with("debug") {
            exe_dir
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("blockchain-core")
        } else {
            // Si ejecutamos desde otro lugar, asumimos que estamos en la raíz del proyecto
            std::env::current_dir().unwrap().join("blockchain-core")
        };

        let logs_dir = blockchain_core_dir.join("logs").join(&session_id);
        let blocks_dir = logs_dir.join("blocks");

        // Crear el directorio logs/timestamp si no existe
        if !logs_dir.exists() {
            fs::create_dir_all(&logs_dir).expect("Failed to create logs directory");
        }

        // Crear el subdirectorio blocks
        if !blocks_dir.exists() {
            fs::create_dir_all(&blocks_dir).expect("Failed to create blocks directory");
        }

        println!("📁 Session logs will be saved to: {}", logs_dir.display());
        println!("📦 Block files will be saved to: {}", blocks_dir.display());

        BlockchainLogger {
            logs_dir: logs_dir.to_string_lossy().to_string(),
            session_id,
        }
    }

    pub fn log_block_creation(&self, block: &Block) {
        let timestamp = Utc::now();
        let log_entry = serde_json::json!({
            "timestamp": timestamp.to_rfc3339(),
            "event": "block_created",
            "block_index": block.index,
            "block_hash": block.hash,
            "previous_hash": block.previous_hash,
            "data": block.data,
            "nonce": block.nonce,
            "difficulty": block.difficulty,
            "block_timestamp": block.timestamp
        });

        self.write_to_file("block_creation.log", &log_entry.to_string());

        // Crear archivo específico para cada bloque en la subcarpeta blocks/
        let block_filename = format!("block_{}.json", block.index);
        self.write_to_subfolder(
            "blocks",
            &block_filename,
            &serde_json::to_string_pretty(&log_entry).unwrap(),
        );
    }

    pub fn log_mining_start(&self, block_index: u64, difficulty: usize) {
        let timestamp = Utc::now();
        let log_entry = format!(
            "[{}] Mining started for block {} with difficulty {}\n",
            timestamp.to_rfc3339(),
            block_index,
            difficulty
        );

        self.write_to_file("mining.log", &log_entry);
        println!("Mining block {}...", block_index);
    }

    pub fn log_mining_complete(&self, block: &Block, duration: std::time::Duration) {
        let timestamp = Utc::now();
        let log_entry = format!(
            "[{}] Block {} mined successfully. Hash: {}, Nonce: {}, Duration: {:?}\n",
            timestamp.to_rfc3339(),
            block.index,
            block.hash,
            block.nonce,
            duration
        );

        self.write_to_file("mining.log", &log_entry);
        println!("Block mined: {} in {:?}", block.hash, duration);
    }

    pub fn log_blockchain_state(&self, blockchain: &Blockchain) {
        let timestamp = Utc::now();
        let blockchain_data = serde_json::json!({
            "timestamp": timestamp.to_rfc3339(),
            "session_id": self.session_id,
            "total_blocks": blockchain.blocks.len(),
            "difficulty": blockchain.difficulty,
            "blocks": blockchain.blocks.iter().map(|block| {
                serde_json::json!({
                    "index": block.index,
                    "hash": block.hash,
                    "previous_hash": block.previous_hash,
                    "data": block.data,
                    "nonce": block.nonce,
                    "difficulty": block.difficulty,
                    "timestamp": block.timestamp
                })
            }).collect::<Vec<_>>()
        });

        self.write_to_file(
            "blockchain_state.json",
            &serde_json::to_string_pretty(&blockchain_data).unwrap(),
        );
    }

    pub fn log_validation_result(&self, is_valid: bool) {
        let timestamp = Utc::now();
        let log_entry = format!(
            "[{}] Blockchain validation result: {}\n",
            timestamp.to_rfc3339(),
            if is_valid { "VALID" } else { "INVALID" }
        );

        self.write_to_file("validation.log", &log_entry);
        println!("Is Blockchain valid? {}", is_valid);
    }

    pub fn log_difficulty_stats(&self, min_diff: usize, max_diff: usize, avg_diff: f64) {
        let timestamp = Utc::now();
        let stats_data = serde_json::json!({
            "timestamp": timestamp.to_rfc3339(),
            "difficulty_stats": {
                "minimum": min_diff,
                "maximum": max_diff,
                "average": avg_diff
            }
        });

        self.write_to_file("difficulty_stats.log", &format!("{}\n", stats_data));
        println!(
            "Difficulty stats: Min={}, Max={}, Avg={:.2}",
            min_diff, max_diff, avg_diff
        );
    }

    fn write_to_file(&self, filename: &str, content: &str) {
        let file_path = Path::new(&self.logs_dir).join(filename);

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .expect(&format!("Failed to open log file: {:?}", file_path));

        file.write_all(content.as_bytes())
            .expect("Failed to write to log file");
    }

    fn write_to_subfolder(&self, subfolder: &str, filename: &str, content: &str) {
        let file_path = Path::new(&self.logs_dir).join(subfolder).join(filename);

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .expect(&format!("Failed to open log file: {:?}", file_path));

        file.write_all(content.as_bytes())
            .expect("Failed to write to log file");
    }

    pub fn create_summary_report(&self, blockchain: &Blockchain) {
        let timestamp = Utc::now();
        let (min_diff, max_diff, avg_diff) = blockchain.get_difficulty_stats();

        let summary = format!(
            "BLOCKCHAIN SUMMARY REPORT\n\
            =========================\n\
            Session ID: {}\n\
            Generated: {}\n\
            Total Blocks: {}\n\
            Current Difficulty: {}\n\
            Difficulty Stats:\n\
              - Minimum: {}\n\
              - Maximum: {}\n\
              - Average: {:.2}\n\
            Blockchain Valid: {}\n\n\
            BLOCK DETAILS:\n\
            {}\n",
            self.session_id,
            timestamp.to_rfc3339(),
            blockchain.blocks.len(),
            blockchain.difficulty,
            min_diff,
            max_diff,
            avg_diff,
            blockchain.is_valid(),
            blockchain
                .blocks
                .iter()
                .map(|block| {
                    format!(
                        "Block {}: Hash={}, Difficulty={}, Nonce={}, Data=\"{}\"",
                        block.index, &block.hash, block.difficulty, block.nonce, &block.data
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        );

        self.write_to_file("summary_report.txt", &summary);
        println!(
            "📄 Summary report saved to {}/summary_report.txt",
            self.logs_dir
        );
    }
}
