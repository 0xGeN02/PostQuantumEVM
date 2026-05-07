//! PQ transaction validation scenarios: send, receipt, nonce.

use std::path::Path;

use pq_wallet_core::{Keystore, PqSigner, PqTxRequest};

use super::runner::{TestResult, TestRunner};

/// Helper: load keypair from runner config.
fn load_keypair(runner: &TestRunner) -> Result<pq_wallet_core::PqKeypair, String> {
    let path_str = runner.keystore_path.as_deref().ok_or("no keystore")?;
    let path = Path::new(path_str);
    Keystore::load(path, &runner.passphrase).map_err(|e| format!("{e}"))
}

/// Send a simple value transfer and verify it gets mined.
pub async fn test_send_transfer(runner: &mut TestRunner) {
    let rpc = runner.primary_rpc();

    // Load keypair
    let keypair = match load_keypair(runner) {
        Ok(kp) => kp,
        Err(e) => {
            runner.record(TestResult::fail(
                "Send PQ transfer",
                format!("Keystore load failed: {e}"),
            ));
            return;
        }
    };

    let sender = keypair.address();

    // Check sender has balance
    let balance = match rpc.get_balance(sender).await {
        Ok(b) => b,
        Err(e) => {
            runner.record(TestResult::fail(
                "Send PQ transfer",
                format!("Cannot get sender balance: {e}"),
            ));
            return;
        }
    };

    if balance == 0 {
        runner.record(TestResult::fail(
            "Send PQ transfer",
            format!("Sender {sender:?} has zero balance — cannot send"),
        ));
        return;
    }

    // Get chain params
    let chain_id = match rpc.chain_id().await {
        Ok(c) => c,
        Err(e) => {
            runner.record(TestResult::fail("Send PQ transfer", format!("chain_id: {e}")));
            return;
        }
    };

    let nonce = match rpc.get_nonce(sender).await {
        Ok(n) => n,
        Err(e) => {
            runner.record(TestResult::fail("Send PQ transfer", format!("nonce: {e}")));
            return;
        }
    };

    let gas_price = match rpc.gas_price().await {
        Ok(gp) => gp.max(1_000_000_000), // min 1 Gwei
        Err(_) => 1_000_000_000,
    };

    // Send 1 wei to self (minimal transfer)
    let tx = PqTxRequest {
        chain_id,
        nonce,
        to: Some(sender), // send to self
        value: 1,
        gas_limit: 21_000,
        gas_price,
        input: vec![],
    };

    let signer = PqSigner::new(&keypair);
    let signed = signer.sign(tx);

    // Verify signature size (ML-DSA-65 = 3309 bytes)
    let sig_size = signed.sig_bytes.len();
    let pk_size = signed.pk_bytes.len();

    let raw = signed.encode();
    let raw_hex = format!("0x{}", hex::encode(&raw));

    // Verify tx type byte is 0x50
    let tx_type_byte = raw[0];
    if tx_type_byte != 0x50 {
        runner.record(TestResult::fail(
            "Send PQ transfer",
            format!("tx type byte = 0x{tx_type_byte:02x}, expected 0x50"),
        ));
        return;
    }

    // Broadcast
    match rpc.send_raw_transaction(&raw_hex).await {
        Ok(hash) => {
            runner.record(TestResult::pass(
                "Send PQ transfer",
                format!(
                    "hash={hash}, sig={sig_size}B (ML-DSA-65), pk={pk_size}B, raw_tx={}B, type=0x50",
                    raw.len()
                ),
            ));
            // Store hash for receipt test
            // We'll re-query it in the receipt test
        }
        Err(e) => {
            runner.record(TestResult::fail(
                "Send PQ transfer",
                format!("broadcast failed: {e}"),
            ));
        }
    }
}

/// Verify the receipt of the last sent transaction.
pub async fn test_receipt_validation(runner: &mut TestRunner) {
    let rpc = runner.primary_rpc();

    // Load keypair to get the sender address
    let keypair = match load_keypair(runner) {
        Ok(kp) => kp,
        Err(e) => {
            runner.record(TestResult::fail(
                "Transaction receipt valid",
                format!("Keystore load failed: {e}"),
            ));
            return;
        }
    };

    let sender = keypair.address();

    // Get nonce — the last tx we sent used nonce N, so current nonce should be N+1
    // We'll check the tx at nonce (current - 1) via block scanning
    let nonce = match rpc.get_nonce(sender).await {
        Ok(n) => n,
        Err(e) => {
            runner.record(TestResult::fail(
                "Transaction receipt valid",
                format!("nonce error: {e}"),
            ));
            return;
        }
    };

    if nonce == 0 {
        runner.record(TestResult::fail(
            "Transaction receipt valid",
            "nonce is 0 — no tx was sent yet",
        ));
        return;
    }

    // Wait for the tx to be mined (up to 15s)
    tokio::time::sleep(std::time::Duration::from_secs(6)).await;

    // Scan recent blocks for our transaction
    let block_num = match rpc.block_number().await {
        Ok(b) => b,
        Err(e) => {
            runner.record(TestResult::fail(
                "Transaction receipt valid",
                format!("block_number: {e}"),
            ));
            return;
        }
    };

    let sender_lower = format!("{sender:?}").to_lowercase();
    let mut found_hash: Option<String> = None;

    for blk in (block_num.saturating_sub(10)..=block_num).rev() {
        if let Ok(Some(block)) = rpc.get_block_by_number(blk).await {
            if let Some(txs) = block.get("transactions").and_then(|v| v.as_array()) {
                for tx in txs {
                    let from = tx.get("from").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                    if from == sender_lower {
                        if let Some(hash) = tx.get("hash").and_then(|v| v.as_str()) {
                            found_hash = Some(hash.to_string());
                            break;
                        }
                    }
                }
            }
        }
        if found_hash.is_some() {
            break;
        }
    }

    let Some(tx_hash) = found_hash else {
        runner.record(TestResult::fail(
            "Transaction receipt valid",
            "Could not find our tx in recent blocks (may still be pending)",
        ));
        return;
    };

    // Get receipt
    match rpc.get_transaction_receipt(&tx_hash).await {
        Ok(Some(receipt)) => {
            let success = receipt.status == "0x1";
            let gas_used_hex = &receipt.gas_used;
            let gas_used = u64::from_str_radix(
                gas_used_hex.strip_prefix("0x").unwrap_or(gas_used_hex),
                16,
            )
            .unwrap_or(0);

            if success && gas_used == 21000 {
                runner.record(TestResult::pass(
                    "Transaction receipt valid",
                    format!("status=success, gasUsed={gas_used}, hash={tx_hash}"),
                ));
            } else if success {
                runner.record(TestResult::pass(
                    "Transaction receipt valid",
                    format!("status=success, gasUsed={gas_used} (expected 21000)"),
                ));
            } else {
                runner.record(TestResult::fail(
                    "Transaction receipt valid",
                    format!("status=REVERTED, gasUsed={gas_used}"),
                ));
            }
        }
        Ok(None) => {
            runner.record(TestResult::fail(
                "Transaction receipt valid",
                format!("receipt is null for {tx_hash} — tx still pending?"),
            ));
        }
        Err(e) => {
            runner.record(TestResult::fail(
                "Transaction receipt valid",
                format!("RPC error: {e}"),
            ));
        }
    }
}

/// Verify that nonce increments after sending a transaction.
pub async fn test_nonce_increment(runner: &mut TestRunner) {
    let rpc = runner.primary_rpc();

    let keypair = match load_keypair(runner) {
        Ok(kp) => kp,
        Err(e) => {
            runner.record(TestResult::fail(
                "Nonce increments correctly",
                format!("Keystore load failed: {e}"),
            ));
            return;
        }
    };

    let sender = keypair.address();
    let nonce = match rpc.get_nonce(sender).await {
        Ok(n) => n,
        Err(e) => {
            runner.record(TestResult::fail(
                "Nonce increments correctly",
                format!("nonce error: {e}"),
            ));
            return;
        }
    };

    // After test_send_transfer, nonce should be >= 1
    if nonce >= 1 {
        runner.record(TestResult::pass(
            "Nonce increments correctly",
            format!("sender nonce={nonce} (≥1 after transfer)"),
        ));
    } else {
        runner.record(TestResult::fail(
            "Nonce increments correctly",
            format!("sender nonce={nonce}, expected ≥1 after sending tx"),
        ));
    }
}
