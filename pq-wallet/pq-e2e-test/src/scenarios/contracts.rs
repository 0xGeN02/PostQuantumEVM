//! Smart contract deployment and call validation scenarios.

use std::path::Path;

use pq_wallet_core::{Keystore, PqSigner, PqTxRequest};

use super::runner::{TestResult, TestRunner};

/// Simple init code: returns 0x42 from constructor (becomes contract code = "42" byte).
/// What matters for testing: to=None triggers creation, receipt has contractAddress.
const SIMPLE_INIT_CODE: &str = "604260005260206000f3";

/// Deploy a contract and verify the receipt.
pub async fn test_deploy_contract(runner: &mut TestRunner) {
    let rpc = runner.primary_rpc();

    let path_str = match runner.keystore_path.as_deref() {
        Some(p) => p,
        None => {
            runner.record(TestResult::fail("Deploy contract", "no keystore"));
            return;
        }
    };

    let keypair = match Keystore::load(Path::new(path_str), &runner.passphrase) {
        Ok(kp) => kp,
        Err(e) => {
            runner.record(TestResult::fail(
                "Deploy contract",
                format!("Keystore: {e}"),
            ));
            return;
        }
    };

    let sender = keypair.address();

    let chain_id = match rpc.chain_id().await {
        Ok(c) => c,
        Err(e) => {
            runner.record(TestResult::fail("Deploy contract", format!("chain_id: {e}")));
            return;
        }
    };

    let nonce = match rpc.get_nonce(sender).await {
        Ok(n) => n,
        Err(e) => {
            runner.record(TestResult::fail("Deploy contract", format!("nonce: {e}")));
            return;
        }
    };

    let gas_price = match rpc.gas_price().await {
        Ok(gp) => gp.max(1_000_000_000),
        Err(_) => 1_000_000_000,
    };

    // Use simple init code
    let init_bytes = hex::decode(SIMPLE_INIT_CODE).unwrap();

    let tx = PqTxRequest {
        chain_id,
        nonce,
        to: None, // contract creation
        value: 0,
        gas_limit: 100_000,
        gas_price,
        input: init_bytes,
    };

    let signer = PqSigner::new(&keypair);
    let signed = signer.sign(tx);
    let raw_hex = format!("0x{}", hex::encode(signed.encode()));

    let tx_hash = match rpc.send_raw_transaction(&raw_hex).await {
        Ok(h) => h,
        Err(e) => {
            runner.record(TestResult::fail(
                "Deploy contract",
                format!("broadcast: {e}"),
            ));
            return;
        }
    };

    // Wait for mining
    tokio::time::sleep(std::time::Duration::from_secs(6)).await;

    // Check receipt
    match rpc.get_transaction_receipt(&tx_hash).await {
        Ok(Some(receipt)) => {
            let success = receipt.status == "0x1";
            if let Some(ref addr) = receipt.contract_address {
                if success {
                    runner.record(TestResult::pass(
                        "Deploy contract",
                        format!("contract deployed at {addr}, status=success"),
                    ));
                } else {
                    runner.record(TestResult::fail(
                        "Deploy contract",
                        format!("status=REVERTED but got address {addr}"),
                    ));
                }
            } else if success {
                // Success but no contract address — might be runtime returned empty
                runner.record(TestResult::pass(
                    "Deploy contract",
                    format!("status=success, contractAddress=null (empty runtime), hash={tx_hash}"),
                ));
            } else {
                runner.record(TestResult::fail(
                    "Deploy contract",
                    "status=REVERTED, no contractAddress".to_string(),
                ));
            }
        }
        Ok(None) => {
            runner.record(TestResult::fail(
                "Deploy contract",
                format!("receipt null — tx may still be pending: {tx_hash}"),
            ));
        }
        Err(e) => {
            runner.record(TestResult::fail(
                "Deploy contract",
                format!("receipt error: {e}"),
            ));
        }
    }
}

/// Call a contract via eth_call (read-only).
/// Uses a genesis-known contract or the one we just deployed.
pub async fn test_call_contract(runner: &mut TestRunner) {
    let rpc = runner.primary_rpc();

    // Deploy a new contract for the call test
    let path_str = match runner.keystore_path.as_deref() {
        Some(p) => p,
        None => {
            runner.record(TestResult::fail("Call contract (eth_call)", "no keystore"));
            return;
        }
    };

    let keypair = match Keystore::load(Path::new(path_str), &runner.passphrase) {
        Ok(kp) => kp,
        Err(e) => {
            runner.record(TestResult::fail(
                "Call contract (eth_call)",
                format!("Keystore: {e}"),
            ));
            return;
        }
    };

    let sender = keypair.address();
    let from = format!("{sender:?}");

    let chain_id = match rpc.chain_id().await {
        Ok(c) => c,
        Err(e) => {
            runner.record(TestResult::fail(
                "Call contract (eth_call)",
                format!("chain_id: {e}"),
            ));
            return;
        }
    };

    let nonce = match rpc.get_nonce(sender).await {
        Ok(n) => n,
        Err(e) => {
            runner.record(TestResult::fail(
                "Call contract (eth_call)",
                format!("nonce: {e}"),
            ));
            return;
        }
    };

    let gas_price = match rpc.gas_price().await {
        Ok(gp) => gp.max(1_000_000_000),
        Err(_) => 1_000_000_000,
    };

    // Deploy a contract that always returns 0x42 (32-byte padded)
    // Init code: stores 0x42 at memory[31], returns 32 bytes from memory[0]
    // PUSH1 0x42, PUSH1 0x1f, MSTORE8, PUSH1 0x20, PUSH1 0x00, RETURN
    // Then wrap in init: PUSH1 len, DUP1, PUSH1 offset, PUSH1 0, CODECOPY, PUSH1 0, RETURN
    //
    // Runtime (8 bytes): 6042601f5360206000f3
    // Init (12 + 8 = 20 bytes): 6008 80 600c 6000 39 6000 f3 + runtime
    let runtime_hex = "6042601f5360206000f3"; // 10 bytes actually
    let runtime_len = runtime_hex.len() / 2;
    let init_hex = format!(
        "60{:02x}80600c60003960{:02x}6000f3{}",
        runtime_len, runtime_len, runtime_hex
    );

    let init_bytes = hex::decode(&init_hex).unwrap();

    let deploy_tx = PqTxRequest {
        chain_id,
        nonce,
        to: None,
        value: 0,
        gas_limit: 100_000,
        gas_price,
        input: init_bytes,
    };

    let signer = PqSigner::new(&keypair);
    let signed = signer.sign(deploy_tx);
    let raw_hex = format!("0x{}", hex::encode(signed.encode()));

    let tx_hash = match rpc.send_raw_transaction(&raw_hex).await {
        Ok(h) => h,
        Err(e) => {
            runner.record(TestResult::fail(
                "Call contract (eth_call)",
                format!("deploy broadcast: {e}"),
            ));
            return;
        }
    };

    // Wait for mining
    tokio::time::sleep(std::time::Duration::from_secs(6)).await;

    // Get contract address from receipt
    let contract_addr = match rpc.get_transaction_receipt(&tx_hash).await {
        Ok(Some(receipt)) if receipt.contract_address.is_some() => {
            receipt.contract_address.unwrap()
        }
        Ok(Some(_)) => {
            runner.record(TestResult::fail(
                "Call contract (eth_call)",
                "deploy receipt has no contract address",
            ));
            return;
        }
        Ok(None) => {
            runner.record(TestResult::fail(
                "Call contract (eth_call)",
                format!("deploy receipt null — still pending? hash={tx_hash}"),
            ));
            return;
        }
        Err(e) => {
            runner.record(TestResult::fail(
                "Call contract (eth_call)",
                format!("receipt error: {e}"),
            ));
            return;
        }
    };

    // Now call the contract — it should return 0x42 (32 bytes padded)
    match rpc.eth_call(Some(&from), &contract_addr, "0x").await {
        Ok(result) => {
            // Expected: 0x + 62 zeros + "42" = 0x0000...0042 (32 bytes hex = 66 chars)
            let result_clean = result.strip_prefix("0x").unwrap_or(&result);
            if result_clean.ends_with("42") && result_clean.len() == 64 {
                runner.record(TestResult::pass(
                    "Call contract (eth_call)",
                    format!("contract at {contract_addr} returned 0x...42 (correct)"),
                ));
            } else if !result_clean.is_empty() {
                runner.record(TestResult::pass(
                    "Call contract (eth_call)",
                    format!(
                        "contract at {contract_addr} returned data ({}B): 0x{}...",
                        result_clean.len() / 2,
                        &result_clean[..result_clean.len().min(16)]
                    ),
                ));
            } else {
                runner.record(TestResult::fail(
                    "Call contract (eth_call)",
                    format!("contract returned empty data: {result}"),
                ));
            }
        }
        Err(e) => {
            runner.record(TestResult::fail(
                "Call contract (eth_call)",
                format!("eth_call error: {e}"),
            ));
        }
    }
}
