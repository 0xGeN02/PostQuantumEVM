mod block;
mod blockchain;
mod pow;
mod logger;

use blockchain::Blockchain;

fn main() {
    let mut blockchain = Blockchain::new();
    
    println!("=== Blockchain (POW) ===");
    
    blockchain.add_block("Alice pays Bob 10 coins".to_string());
    blockchain.add_block("Bob pays Charlie 5 coins".to_string());
    blockchain.add_block("Charlie pays Dave 3 coins".to_string());
    blockchain.add_block("Dave pays Eve 2 coins".to_string());
    blockchain.add_block("Eve pays Frank 1 coin".to_string());
    blockchain.add_block("Frank pays Alice 4 coins".to_string());

    println!("\n=== Blockchain State ===");
    for block in &blockchain.blocks {
        println!(
            "Block {}: Difficulty={}, Hash={}, Nonce={}", 
            block.index, 
            block.difficulty,
            &block.hash,
            block.nonce
        );
    }

    // Log the current blockchain state
    blockchain.log_blockchain_state();

    println!("\n=== Validation ===");
    let _is_valid = blockchain.log_validation_result();

    println!("\n=== Difficulty Statistics ===");
    blockchain.log_difficulty_stats();

    // Create a comprehensive summary report
    blockchain.create_summary_report();
    
    println!("\n✅ All blockchain data has been logged to the session directory");
}