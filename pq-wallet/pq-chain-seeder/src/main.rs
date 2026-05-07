//! pq-chain-seeder — Populate a PostQuantumEVM chain with demo transactions.
//!
//! Sends a pre-defined sequence of PQ transactions to a running node:
//! - Simple qETH transfers (varying amounts)
//! - Contract deployment (SimpleStorage)
//! - Contract interactions (store/retrieve values)
//!
//! Usage:
//!   pq-seed --rpc http://localhost:8545 --keystore ../keystore.json --passphrase <pass>

use std::path::PathBuf;
use std::time::Duration;

use alloy_primitives::Address;
use anyhow::{Context, Result, bail};
use clap::Parser;
use pq_wallet_core::{Keystore, PqSigner, PqTxRequest, RpcClient};

// ─── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "pq-seed", about = "Seed a PostQuantumEVM chain with demo transactions")]
struct Cli {
    /// RPC endpoint URL
    #[arg(long, default_value = "http://localhost:8545")]
    rpc: String,

    /// Path to the keystore JSON file
    #[arg(long, default_value = "../keystore.json")]
    keystore: PathBuf,

    /// Passphrase for the keystore
    #[arg(long, env = "PQ_PASSPHRASE")]
    passphrase: String,

    /// Number of transfer transactions to send
    #[arg(long, default_value = "10")]
    transfers: usize,

    /// Whether to deploy the demo contract
    #[arg(long, default_value = "true")]
    deploy_contract: bool,

    /// Number of contract calls to make (store operations)
    #[arg(long, default_value = "5")]
    contract_calls: usize,

    /// Seconds to wait between transactions (for block mining)
    #[arg(long, default_value = "2")]
    tx_delay: u64,
}

// ─── Constants ───────────────────────────────────────────────────────────────

/// Chain ID for PostQuantumEVM
const CHAIN_ID: u64 = 20561;

/// Gas limit for simple transfers
const TRANSFER_GAS: u64 = 21_000;

/// Gas limit for contract deployment
const DEPLOY_GAS: u64 = 200_000;

/// Gas limit for contract calls
const CALL_GAS: u64 = 100_000;

/// Known recipient addresses for demo transfers (deterministic, look good in TUI)
const DEMO_RECIPIENTS: &[&str] = &[
    "0x1111111111111111111111111111111111111111",
    "0x2222222222222222222222222222222222222222",
    "0x3333333333333333333333333333333333333333",
    "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
    "0xcafecafecafecafecafecafecafecafecafecafe",
];

/// Function selector for store(uint256): keccak256("store(uint256)")[:4]
const STORE_SELECTOR: &str = "6057361d";

// ─── Main ────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    println!("=== PostQuantumEVM Chain Seeder ===\n");

    // Load keystore
    println!("[1/4] Loading keystore from {:?}...", cli.keystore);
    let keypair = Keystore::load(&cli.keystore, &cli.passphrase)
        .context("Failed to load keystore. Check path and passphrase.")?;
    let address = keypair.address();
    println!("       Sender: {address:?}");

    // Connect to node
    println!("[2/4] Connecting to {}...", cli.rpc);
    let rpc = RpcClient::new(&cli.rpc);
    let chain_id = rpc.chain_id().await.context("Failed to get chain ID")?;
    if chain_id != CHAIN_ID {
        bail!("Wrong chain ID: expected {CHAIN_ID}, got {chain_id}");
    }
    let balance = rpc.get_balance(address).await?;
    let balance_eth = balance as f64 / 1e18;
    println!("       Chain ID: {chain_id}");
    println!("       Balance: {balance_eth:.4} qETH");

    if balance < 1_000_000_000_000_000_000 {
        bail!("Insufficient balance for seeding (need at least 1 qETH)");
    }

    let mut nonce = rpc.get_nonce(address).await?;
    let gas_price = rpc.gas_price().await?.max(1_000_000_000); // min 1 gwei
    println!("       Starting nonce: {nonce}");
    println!("       Gas price: {} gwei\n", gas_price / 1_000_000_000);

    let signer = PqSigner::new(&keypair);
    let delay = Duration::from_secs(cli.tx_delay);

    // Phase 1: Transfers
    println!("[3/4] Sending {} transfer transactions...", cli.transfers);
    for i in 0..cli.transfers {
        let recipient = DEMO_RECIPIENTS[i % DEMO_RECIPIENTS.len()];
        let recipient_addr = parse_address(recipient)?;
        // Vary amounts: 0.01, 0.05, 0.1, 0.5, 1.0 qETH pattern
        let amounts = [
            10_000_000_000_000_000u128,   // 0.01 qETH
            50_000_000_000_000_000,       // 0.05 qETH
            100_000_000_000_000_000,      // 0.1 qETH
            500_000_000_000_000_000,      // 0.5 qETH
            1_000_000_000_000_000_000,    // 1.0 qETH
        ];
        let value = amounts[i % amounts.len()];

        let tx = PqTxRequest {
            chain_id: CHAIN_ID,
            nonce,
            to: Some(recipient_addr),
            value,
            gas_limit: TRANSFER_GAS,
            gas_price,
            input: vec![],
        };

        let signed = signer.sign(tx);
        let raw_hex = format!("0x{}", hex::encode(signed.encode()));
        let tx_hash = rpc.send_raw_transaction(&raw_hex).await
            .with_context(|| format!("Transfer #{} failed", i + 1))?;

        let value_eth = value as f64 / 1e18;
        println!("       tx #{:<2} -> {} | {:.2} qETH | {}", i + 1, &recipient[..10], value_eth, tx_hash);
        nonce += 1;

        tokio::time::sleep(delay).await;
    }

    // Phase 2: Contract deployment
    let mut contract_address: Option<String> = None;
    if cli.deploy_contract {
        println!("\n[4/4] Deploying SimpleStorage contract...");

        // Use a simple contract that actually works:
        // PUSH1 0x42 PUSH1 0x00 SSTORE (stores 0x42 at slot 0)
        // Then returns a minimal runtime that supports store/retrieve
        let init_code = create_demo_contract_init();

        let tx = PqTxRequest {
            chain_id: CHAIN_ID,
            nonce,
            to: None, // contract creation
            value: 0,
            gas_limit: DEPLOY_GAS,
            gas_price,
            input: init_code,
        };

        let signed = signer.sign(tx);
        let raw_hex = format!("0x{}", hex::encode(signed.encode()));
        let tx_hash = rpc.send_raw_transaction(&raw_hex).await
            .context("Contract deployment failed")?;
        println!("       Deploy tx: {tx_hash}");
        nonce += 1;

        // Wait for receipt
        tokio::time::sleep(delay).await;
        for _ in 0..10 {
            if let Ok(Some(receipt)) = rpc.get_transaction_receipt(&tx_hash).await {
                if receipt.status == "0x1" {
                    contract_address = receipt.contract_address.clone();
                    println!("       Contract deployed at: {}", contract_address.as_deref().unwrap_or("unknown"));
                    break;
                } else {
                    println!("       WARNING: Deploy tx reverted");
                    break;
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        // Phase 3: Contract calls (store different values)
        if let Some(ref addr) = contract_address {
            println!("       Sending {} store() calls...", cli.contract_calls);
            for i in 0..cli.contract_calls {
                let value_to_store = (i as u64 + 1) * 100; // 100, 200, 300, ...
                let calldata = encode_store_call(value_to_store);

                let tx = PqTxRequest {
                    chain_id: CHAIN_ID,
                    nonce,
                    to: Some(parse_address(addr)?),
                    value: 0,
                    gas_limit: CALL_GAS,
                    gas_price,
                    input: calldata,
                };

                let signed = signer.sign(tx);
                let raw_hex = format!("0x{}", hex::encode(signed.encode()));
                let tx_hash = rpc.send_raw_transaction(&raw_hex).await
                    .with_context(|| format!("store() call #{} failed", i + 1))?;
                println!("       store({}) -> {}", value_to_store, tx_hash);
                nonce += 1;

                tokio::time::sleep(delay).await;
            }

            // Verify final stored value with eth_call
            let retrieve_data = "0x2e64cec1"; // retrieve() selector
            let from_hex = format!("{address:?}");
            match rpc.eth_call(Some(&from_hex), addr, retrieve_data).await {
                Ok(result) => {
                    let stored = parse_uint256_result(&result);
                    println!("       retrieve() = {} (verified via eth_call)", stored);
                }
                Err(e) => println!("       retrieve() failed: {e}"),
            }
        }
    }

    // Summary
    println!("\n=== Seeding Complete ===");
    println!("  Transfers sent: {}", cli.transfers);
    if let Some(addr) = &contract_address {
        println!("  Contract deployed: {addr}");
        println!("  Contract calls: {}", cli.contract_calls);
    }
    println!("  Final nonce: {nonce}");
    let new_balance = rpc.get_balance(address).await.unwrap_or(0);
    println!("  Remaining balance: {:.4} qETH", new_balance as f64 / 1e18);
    println!("========================\n");

    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn parse_address(s: &str) -> Result<Address> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).context("invalid hex address")?;
    if bytes.len() != 20 {
        bail!("address must be 20 bytes, got {}", bytes.len());
    }
    Ok(Address::from_slice(&bytes))
}

/// Encode a `store(uint256)` call.
fn encode_store_call(value: u64) -> Vec<u8> {
    let selector = hex::decode(STORE_SELECTOR).unwrap();
    let mut calldata = Vec::with_capacity(36);
    calldata.extend_from_slice(&selector);
    // ABI-encode uint256: 32 bytes, big-endian, zero-padded
    let mut padded = [0u8; 32];
    padded[24..].copy_from_slice(&value.to_be_bytes());
    calldata.extend_from_slice(&padded);
    calldata
}

/// Create a minimal demo contract init code.
///
/// This creates a contract with:
/// - store(uint256 value): SSTORE at slot 0
/// - retrieve() returns (uint256): SLOAD from slot 0
///
/// Hand-crafted EVM bytecode (no Solidity compiler needed):
fn create_demo_contract_init() -> Vec<u8> {
    // Runtime bytecode:
    //   CALLDATASIZE < 4 → fallback (revert)
    //   CALLDATALOAD(0) >> 224 → function selector
    //   if selector == 0x6057361d → store(uint256)
    //   if selector == 0x2e64cec1 → retrieve()
    //
    // store(uint256):
    //   CALLDATALOAD(4) → value
    //   SSTORE(0, value)
    //   STOP
    //
    // retrieve():
    //   SLOAD(0)
    //   PUSH1 0x00 MSTORE
    //   PUSH1 0x20 PUSH1 0x00 RETURN
    let runtime: Vec<u8> = vec![
        // Check calldatasize >= 4
        0x60, 0x04,             // PUSH1 4
        0x36,                   // CALLDATASIZE
        0x10,                   // LT (calldatasize < 4)
        0x60, 0x40,             // PUSH1 0x40 (revert offset)
        0x57,                   // JUMPI → revert if calldatasize < 4
        // Get selector: calldataload(0) >> 224
        0x60, 0x00,             // PUSH1 0
        0x35,                   // CALLDATALOAD
        0x60, 0xe0,             // PUSH1 224
        0x1c,                   // SHR
        // Check if selector == 0x6057361d (store)
        0x80,                   // DUP1
        0x63, 0x60, 0x57, 0x36, 0x1d,  // PUSH4 0x6057361d
        0x14,                   // EQ
        0x60, 0x25,             // PUSH1 0x25 (store jump dest)
        0x57,                   // JUMPI
        // Check if selector == 0x2e64cec1 (retrieve)
        0x63, 0x2e, 0x64, 0xce, 0xc1,  // PUSH4 0x2e64cec1
        0x14,                   // EQ
        0x60, 0x31,             // PUSH1 0x31 (retrieve jump dest)
        0x57,                   // JUMPI
        // Fallback: revert
        0x60, 0x00,             // PUSH1 0
        0x60, 0x00,             // PUSH1 0
        0xfd,                   // REVERT
        // store(uint256): offset 0x25 = 37
        0x5b,                   // JUMPDEST
        0x60, 0x04,             // PUSH1 4
        0x35,                   // CALLDATALOAD (load value from calldata[4:36])
        0x60, 0x00,             // PUSH1 0 (storage slot)
        0x55,                   // SSTORE
        0x00,                   // STOP
        // retrieve(): offset 0x31 = 49
        0x5b,                   // JUMPDEST
        0x60, 0x00,             // PUSH1 0 (storage slot)
        0x54,                   // SLOAD
        0x60, 0x00,             // PUSH1 0 (memory offset)
        0x52,                   // MSTORE
        0x60, 0x20,             // PUSH1 32 (return size)
        0x60, 0x00,             // PUSH1 0 (return offset)
        0xf3,                   // RETURN
        // revert target: offset 0x40 = 64
        0x5b,                   // JUMPDEST
        0x60, 0x00,             // PUSH1 0
        0x60, 0x00,             // PUSH1 0
        0xfd,                   // REVERT
    ];

    // Init code: copy runtime to memory and return it
    let runtime_len = runtime.len();
    let mut init = Vec::new();

    // PUSH1 <runtime_len>
    init.push(0x60);
    init.push(runtime_len as u8);
    // DUP1 (for RETURN later)
    init.push(0x80);
    // PUSH1 <init_code_total_len> (offset where runtime starts in the deployed code)
    // init code = these bytes + CODECOPY + RETURN overhead
    // We'll calculate: init prefix is 10 bytes total
    let init_prefix_len: u8 = 10;
    init.push(0x60);
    init.push(init_prefix_len);
    // PUSH1 0x00 (destOffset in memory)
    init.push(0x60);
    init.push(0x00);
    // CODECOPY (copies runtime into memory[0..runtime_len])
    init.push(0x39);
    // PUSH1 0x00 (offset for RETURN)
    init.push(0x60);
    init.push(0x00);
    // RETURN (returns memory[0..runtime_len])
    init.push(0xf3);

    assert_eq!(init.len(), init_prefix_len as usize, "init prefix length mismatch");

    // Append runtime after init code
    init.extend_from_slice(&runtime);
    init
}

/// Parse a uint256 hex result from eth_call into a u64.
fn parse_uint256_result(hex_str: &str) -> u64 {
    let s = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    // Take last 16 hex chars (8 bytes = u64 range)
    if s.len() >= 16 {
        let last16 = &s[s.len() - 16..];
        u64::from_str_radix(last16, 16).unwrap_or(0)
    } else {
        u64::from_str_radix(s, 16).unwrap_or(0)
    }
}
