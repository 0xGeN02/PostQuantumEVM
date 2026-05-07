//! pq-e2e — End-to-End validation for PostQuantumEVM.
//!
//! Connects to a running PQ-EVM node cluster and validates:
//!   1. Chain identity (chain ID, genesis, native currency)
//!   2. PoA consensus (block production, validator rotation)
//!   3. EIP-1559 fee model (base fee adjustment, priority fees)
//!   4. PQ transactions (type 0x50, ML-DSA-65 signatures)
//!   5. Value transfers (balance changes, gas accounting)
//!   6. Contract deployment (receipt with contract_address)
//!   7. Contract calls (eth_call, state reads)
//!   8. Multi-node consistency (all validators see same state)
//!
//! Usage:
//!   pq-e2e --rpc http://localhost:8545
//!   pq-e2e --rpc http://localhost:8545,http://localhost:8546,http://localhost:8547
//!   pq-e2e --rpc http://localhost:8545 --keystore ./keystore.json --passphrase test

mod scenarios;

use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;

use scenarios::TestRunner;

/// PostQuantumEVM End-to-End Validation Tool
#[derive(Parser, Debug)]
#[command(name = "pq-e2e", about = "E2E validation for PostQuantumEVM")]
struct Args {
    /// Comma-separated list of RPC endpoints to validate against.
    #[arg(long, default_value = "http://localhost:8545")]
    rpc: String,

    /// Path to a funded keystore file (ML-DSA-65).
    /// Required for transaction tests (send, deploy).
    #[arg(long)]
    keystore: Option<String>,

    /// Passphrase for the keystore.
    #[arg(long, default_value = "")]
    passphrase: String,

    /// Only run read-only validation (no transactions sent).
    /// Useful for validating an already-seeded chain.
    #[arg(long, default_value_t = false)]
    readonly: bool,

    /// Verbose output (show individual check details).
    #[arg(long, short, default_value_t = false)]
    verbose: bool,

    /// Wait up to N seconds for the node to become available.
    #[arg(long, default_value_t = 30)]
    wait: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let rpc_endpoints: Vec<String> = args.rpc.split(',').map(|s| s.trim().to_string()).collect();

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║        PostQuantumEVM — End-to-End Validation               ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Chain ID:    20561                                         ║");
    println!("║  Consensus:   PoA (ML-DSA-65 sealed)                        ║");
    println!("║  Tx type:     0x50 (PQ envelope)                            ║");
    println!("║  Address:     SHAKE-256(pk)[12..]                           ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let start = Instant::now();

    let mut runner = TestRunner::new(
        rpc_endpoints,
        args.keystore,
        args.passphrase,
        args.readonly,
        args.verbose,
        args.wait,
    );

    runner.run().await.context("E2E validation failed")?;

    let elapsed = start.elapsed();
    println!();
    println!(
        "═══ All tests passed in {:.2}s ═══",
        elapsed.as_secs_f64()
    );
    println!();

    Ok(())
}
