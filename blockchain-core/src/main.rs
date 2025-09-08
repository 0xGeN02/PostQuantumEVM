mod block;
mod blockchain;
mod consensus;
mod logger;

use blockchain::Blockchain;
use consensus::ConsensusType;

fn main() {
    println!("🚀 Post-Quantum Cryptography Blockchain with Configurable Consensus");
    println!("===================================================================\n");

    // Demostrar diferentes algoritmos de consenso
    demo_consensus_algorithms();

    // Benchmark de rendimiento
    println!("\n🔬 Running Consensus Algorithm Benchmark...");
    benchmark_algorithms();

    // Demostrar cambio dinámico de consenso
    println!("\n🔄 Demonstrating Dynamic Consensus Switching...");
    demo_consensus_switching();
}

fn demo_consensus_algorithms() {
    let consensus_types = vec![
        (
            "Proof of Work",
            ConsensusType::ProofOfWork { difficulty: 3 },
        ),
        (
            "Proof of Stake",
            ConsensusType::ProofOfStake {
                minimum_stake: 1000,
            },
        ),
        (
            "Proof of Authority",
            ConsensusType::ProofOfAuthority {
                validators: vec![
                    "alice".to_string(),
                    "bob".to_string(),
                    "charlie".to_string(),
                ],
            },
        ),
        (
            "Proof of History",
            ConsensusType::ProofOfHistory {
                vdf_iterations: 500,
            },
        ),
        (
            "Proof of Elapsed Time",
            ConsensusType::ProofOfElapsedTime {
                wait_time_config: 100,
            },
        ),
    ];

    for (name, consensus_type) in consensus_types {
        println!("\n🔹 Testing {} Algorithm", name);
        println!("Description: {}", consensus_type.description());

        match Blockchain::new_with_consensus(consensus_type.clone()) {
            Ok(mut blockchain) => {
                println!("✅ Blockchain initialized with {}", name);

                // Añadir algunos bloques
                let transactions = vec![
                    "Alice pays Bob 10 coins",
                    "Bob pays Charlie 5 coins",
                    "Charlie pays Dave 3 coins",
                ];

                for (i, tx) in transactions.iter().enumerate() {
                    match blockchain.add_block(tx.to_string()) {
                        Ok(result) => {
                            println!("  Block {}: ✅ Added in {:?}", i + 1, result.execution_time);
                            if let Some(energy) = result.energy_cost {
                                println!("    Energy cost: {:.6}", energy);
                            }
                        }
                        Err(e) => println!("  Block {}: ❌ Failed - {}", i + 1, e),
                    }
                }

                // Mostrar características del algoritmo
                let characteristics = consensus_type.characteristics();
                println!("  Characteristics:");
                for (key, value) in characteristics {
                    println!("    {}: {}", key, value);
                }

                // Validar blockchain
                let is_valid = blockchain.is_valid();
                println!(
                    "  Blockchain validation: {}",
                    if is_valid { "✅ Valid" } else { "❌ Invalid" }
                );

                // Mostrar estadísticas
                blockchain.log_consensus_statistics();
            }
            Err(e) => println!("❌ Failed to initialize {}: {}", name, e),
        }

        println!("{}", "─".repeat(50));
    }
}

fn benchmark_algorithms() {
    let test_data = vec![
        "Transaction 1".to_string(),
        "Transaction 2".to_string(),
        "Transaction 3".to_string(),
    ];

    match Blockchain::new() {
        blockchain => match blockchain.benchmark_consensus_algorithms(test_data) {
            Ok(results) => {
                println!("\n📊 Benchmark Results:");
                println!(
                    "{:<30} {:<15} {:<15}",
                    "Algorithm", "Time (ms)", "Energy Cost"
                );
                println!("{}", "─".repeat(60));

                for (name, duration, energy) in results {
                    println!("{:<30} {:<15} {:<15.6}", name, duration.as_millis(), energy);
                }
            }
            Err(e) => println!("❌ Benchmark failed: {}", e),
        },
    }
}

fn demo_consensus_switching() {
    let mut blockchain = Blockchain::new();
    println!("🔧 Initial consensus: {}", blockchain.consensus_type.name());

    // Añadir bloques con PoW
    println!("\n📦 Adding blocks with Proof of Work...");
    for i in 1..=3 {
        match blockchain.add_block(format!("PoW Transaction {}", i)) {
            Ok(_) => println!("  ✅ Block {} added with PoW", i),
            Err(e) => println!("  ❌ Block {} failed: {}", i, e),
        }
    }

    // Cambiar a PoS
    let pos_consensus = ConsensusType::ProofOfStake { minimum_stake: 500 };
    match blockchain.switch_consensus(pos_consensus) {
        Ok(_) => {
            println!("\n🔄 Switched to Proof of Stake");

            // Añadir más bloques con PoS
            println!("\n📦 Adding blocks with Proof of Stake...");
            for i in 4..=6 {
                match blockchain.add_block(format!("PoS Transaction {}", i)) {
                    Ok(_) => println!("  ✅ Block {} added with PoS", i),
                    Err(e) => println!("  ❌ Block {} failed: {}", i, e),
                }
            }
        }
        Err(e) => println!("❌ Failed to switch consensus: {}", e),
    }

    // Cambiar a PoA
    let poa_consensus = ConsensusType::ProofOfAuthority {
        validators: vec!["authority1".to_string(), "authority2".to_string()],
    };
    match blockchain.switch_consensus(poa_consensus) {
        Ok(_) => {
            println!("\n🔄 Switched to Proof of Authority");

            println!("\n📦 Adding blocks with Proof of Authority...");
            for i in 7..=9 {
                match blockchain.add_block(format!("PoA Transaction {}", i)) {
                    Ok(_) => println!("  ✅ Block {} added with PoA", i),
                    Err(e) => println!("  ❌ Block {} failed: {}", i, e),
                }
            }
        }
        Err(e) => println!("❌ Failed to switch consensus: {}", e),
    }

    println!("\n📋 Final Blockchain State:");
    for (i, block) in blockchain.blocks.iter().enumerate() {
        let default_type = "Unknown".to_string();
        let consensus_type = block
            .get_consensus_data("algorithm_name")
            .unwrap_or(&default_type);
        println!(
            "  Block {}: {} - Hash: {}",
            i,
            consensus_type,
            &block.hash[..16]
        );
    }

    // Validación final
    let is_valid = blockchain.is_valid();
    println!(
        "\n🔍 Final validation: {}",
        if is_valid { "✅ Valid" } else { "❌ Invalid" }
    );

    // Estadísticas finales
    blockchain.log_consensus_statistics();
    blockchain.create_summary_report();
}
