use blockchain_core::blockchain::Blockchain;
use blockchain_core::consensus::*;
use std::time::Instant;

fn main() {
    println!("🔧 Advanced Consensus Algorithm Configuration Demo");
    println!("==================================================\n");

    // Demo de configuración específica para cada algoritmo
    demo_algorithm_specific_configuration();

    // Demo de algoritmos más complejos
    demo_advanced_algorithms();

    // Demo de métricas y análisis
    demo_consensus_analytics();
}

fn demo_algorithm_specific_configuration() {
    println!("🔧 Algorithm-Specific Configuration Demo\n");

    // Configurar PoW con parámetros específicos
    println!("🔹 Configuring Proof of Work");
    let pow_config = ConsensusType::ProofOfWork { difficulty: 5 };
    match Blockchain::new_with_consensus(pow_config) {
        Ok(mut blockchain) => {
            println!("✅ PoW blockchain created with difficulty 5");

            // Medir tiempo de minado
            let start = Instant::now();
            match blockchain.add_block("High difficulty transaction".to_string()) {
                Ok(_) => {
                    let duration = start.elapsed();
                    println!("  ⏱️  Block mined in: {:?}", duration);
                }
                Err(e) => println!("  ❌ Mining failed: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to create PoW blockchain: {}", e),
    }

    // Configurar PoS con múltiples validadores
    println!("\n🔹 Configuring Proof of Stake");
    let pos_config = ConsensusType::ProofOfStake {
        minimum_stake: 10000,
    };
    match Blockchain::new_with_consensus(pos_config) {
        Ok(mut blockchain) => {
            println!("✅ PoS blockchain created with minimum stake: 10,000");

            match blockchain.add_block("Large stake transaction".to_string()) {
                Ok(result) => {
                    println!("  ⏱️  Block created in: {:?}", result.execution_time);
                    if let Some(energy) = result.energy_cost {
                        println!("  ⚡ Energy consumption: {:.6} units", energy);
                    }
                }
                Err(e) => println!("  ❌ Block creation failed: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to create PoS blockchain: {}", e),
    }

    // Configurar PoA con autoridades específicas
    println!("\n🔹 Configuring Proof of Authority");
    let authorities = vec![
        "alice@company.com".to_string(),
        "bob@company.com".to_string(),
        "charlie@company.com".to_string(),
        "dave@company.com".to_string(),
    ];
    let poa_config = ConsensusType::ProofOfAuthority {
        validators: authorities.clone(),
    };

    match Blockchain::new_with_consensus(poa_config) {
        Ok(mut blockchain) => {
            println!(
                "✅ PoA blockchain created with {} authorities",
                authorities.len()
            );
            println!("  Authorities: {:?}", authorities);

            for i in 1..=5 {
                match blockchain.add_block(format!("Authority transaction {}", i)) {
                    Ok(result) => {
                        if let Some(authority) = result.proof_data.get("authority_address") {
                            println!("  📋 Block {} validated by: {}", i, authority);
                        }
                    }
                    Err(e) => println!("  ❌ Block {} failed: {}", i, e),
                }
            }
        }
        Err(e) => println!("❌ Failed to create PoA blockchain: {}", e),
    }
}

fn demo_advanced_algorithms() {
    println!("\n🚀 Advanced Consensus Algorithms Demo\n");

    // Demo de Proof of Burn
    println!("🔹 Proof of Burn Demo");
    let pob_config = ConsensusType::ProofOfBurn { burn_amount: 1000 };
    match Blockchain::new_with_consensus(pob_config) {
        Ok(mut blockchain) => {
            println!("✅ PoB blockchain created (burn amount: 1,000 coins)");

            match blockchain.add_block("Burned coins for mining rights".to_string()) {
                Ok(result) => {
                    if let Some(burned) = result.proof_data.get("total_burned") {
                        println!("  🔥 Total coins burned: {}", burned);
                    }
                    println!("  ⏱️  Block time: {:?}", result.execution_time);
                }
                Err(e) => println!("  ❌ PoB mining failed: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to create PoB blockchain: {}", e),
    }

    // Demo de Proof of Capacity
    println!("\n🔹 Proof of Capacity Demo");
    let poc_config = ConsensusType::ProofOfCapacity {
        storage_requirement: 100,
    };
    match Blockchain::new_with_consensus(poc_config) {
        Ok(mut blockchain) => {
            println!("✅ PoC blockchain created (storage: 100 GB required)");

            match blockchain.add_block("Storage-based mining".to_string()) {
                Ok(result) => {
                    if let Some(capacity) = result.proof_data.get("total_capacity_gb") {
                        println!("  💾 Total storage capacity: {} GB", capacity);
                    }
                    if let Some(plots) = result.proof_data.get("total_plots") {
                        println!("  📊 Active storage plots: {}", plots);
                    }
                }
                Err(e) => println!("  ❌ PoC mining failed: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to create PoC blockchain: {}", e),
    }

    // Demo de pBFT
    println!("\n🔹 Practical Byzantine Fault Tolerance Demo");
    let pbft_config = ConsensusType::PracticalByzantineFaultTolerance {
        node_count: 7,
        fault_tolerance: 0.3,
    };
    match Blockchain::new_with_consensus(pbft_config) {
        Ok(mut blockchain) => {
            println!("✅ pBFT blockchain created (7 nodes, 30% fault tolerance)");

            match blockchain.add_block("Byzantine fault tolerant transaction".to_string()) {
                Ok(result) => {
                    if let Some(honest) = result.proof_data.get("honest_nodes") {
                        println!("  ✅ Honest nodes: {}", honest);
                    }
                    if let Some(faulty) = result.proof_data.get("faulty_nodes") {
                        println!("  ⚠️  Faulty nodes: {}", faulty);
                    }
                    if let Some(primary) = result.proof_data.get("primary_node") {
                        println!("  👑 Primary node: {}", primary);
                    }
                }
                Err(e) => println!("  ❌ pBFT consensus failed: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to create pBFT blockchain: {}", e),
    }
}

fn demo_consensus_analytics() {
    println!("\n📊 Consensus Analytics and Comparison\n");

    let algorithms = vec![
        ("PoW (Low)", ConsensusType::ProofOfWork { difficulty: 2 }),
        ("PoW (High)", ConsensusType::ProofOfWork { difficulty: 4 }),
        (
            "PoS",
            ConsensusType::ProofOfStake {
                minimum_stake: 1000,
            },
        ),
        (
            "PoA",
            ConsensusType::ProofOfAuthority {
                validators: vec!["auth1".to_string(), "auth2".to_string()],
            },
        ),
        (
            "PoH",
            ConsensusType::ProofOfHistory {
                vdf_iterations: 100,
            },
        ),
    ];

    println!("🔬 Performance Comparison:");
    println!(
        "{:<15} {:<12} {:<12} {:<15} {:<10}",
        "Algorithm", "Blocks", "Avg Time", "Energy", "Efficiency"
    );
    println!("{}", "─".repeat(70));

    for (name, consensus_type) in algorithms {
        match Blockchain::new_with_consensus(consensus_type.clone()) {
            Ok(mut blockchain) => {
                let mut total_time = 0u128;
                let mut total_energy = 0.0;
                let block_count = 3;

                for i in 1..=block_count {
                    let start = Instant::now();
                    match blockchain.add_block(format!("Test transaction {}", i)) {
                        Ok(result) => {
                            total_time += result.execution_time.as_millis();
                            if let Some(energy) = result.energy_cost {
                                total_energy += energy;
                            }
                        }
                        Err(_) => continue,
                    }
                }

                let avg_time = total_time / block_count;
                let avg_energy = total_energy / block_count as f64;

                // Obtener eficiencia energética
                let efficiency = match blockchain.get_consensus_info() {
                    Ok(info) => info
                        .get("energy_efficiency")
                        .and_then(|s| s.parse::<f64>().ok())
                        .map(|e| format!("{:.2}%", e * 100.0))
                        .unwrap_or("N/A".to_string()),
                    Err(_) => "N/A".to_string(),
                };

                println!(
                    "{:<15} {:<12} {:<12} {:<15.6} {:<10}",
                    name,
                    block_count,
                    format!("{}ms", avg_time),
                    avg_energy,
                    efficiency
                );
            }
            Err(_) => {
                println!(
                    "{:<15} {:<12} {:<12} {:<15} {:<10}",
                    name, "Failed", "N/A", "N/A", "N/A"
                );
            }
        }
    }

    println!("\n📈 Algorithm Characteristics Summary:");
    for (name, consensus_type) in &[
        (
            "Proof of Work",
            ConsensusType::ProofOfWork { difficulty: 4 },
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
                validators: vec!["auth1".to_string()],
            },
        ),
    ] {
        println!("\n🔸 {}:", name);
        println!("   Description: {}", consensus_type.description());

        let characteristics = consensus_type.characteristics();
        for (key, value) in characteristics {
            println!("   {}: {}", key, value);
        }
    }

    println!("\n✨ Demo completed! Check the logs for detailed information.");
}
