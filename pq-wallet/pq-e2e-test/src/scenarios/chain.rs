//! Chain identity validation scenarios.

use super::runner::{TestResult, TestRunner};

/// Verify chain ID matches PostQuantumEVM (20561).
pub async fn test_chain_id(runner: &mut TestRunner) {
    let rpc = runner.primary_rpc();
    match rpc.chain_id().await {
        Ok(chain_id) => {
            if chain_id == 20561 {
                runner.record(TestResult::pass(
                    "Chain ID = 20561",
                    format!("chain_id={chain_id} (0x{chain_id:x})"),
                ));
            } else {
                runner.record(TestResult::fail(
                    "Chain ID = 20561",
                    format!("expected 20561, got {chain_id}"),
                ));
            }
        }
        Err(e) => {
            runner.record(TestResult::fail("Chain ID = 20561", format!("RPC error: {e}")));
        }
    }
}

/// Verify genesis pre-funded accounts have expected balances.
pub async fn test_genesis_accounts(runner: &mut TestRunner) {
    let rpc = runner.primary_rpc();

    // Expected: 10,000 qETH = 0x21e19e0c9bab2400000 wei
    let expected_balance: u128 = 0x21e19e0c9bab2400000;
    let test_addr = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"; // hardhat #0

    let addr = alloy_primitives::Address::parse_checksummed(test_addr, None)
        .or_else(|_| {
            let clean = test_addr.strip_prefix("0x").unwrap_or(test_addr);
            hex::decode(clean)
                .ok()
                .filter(|b| b.len() == 20)
                .map(|b| alloy_primitives::Address::from_slice(&b))
                .ok_or(())
        })
        .ok();

    let Some(addr) = addr else {
        runner.record(TestResult::fail(
            "Genesis account funded",
            "Failed to parse test address",
        ));
        return;
    };

    match rpc.get_balance(addr).await {
        Ok(balance) => {
            // Balance may be less if txs have been sent, but should be > 0
            if balance > 0 {
                let qeth = balance as f64 / 1e18;
                let detail = if balance == expected_balance {
                    format!("balance={qeth:.2} qETH (exact genesis amount)")
                } else {
                    format!("balance={qeth:.6} qETH (genesis minus spent gas/value)")
                };
                runner.record(TestResult::pass("Genesis account funded", detail));
            } else {
                runner.record(TestResult::fail(
                    "Genesis account funded",
                    format!("balance=0 for {test_addr}"),
                ));
            }
        }
        Err(e) => {
            runner.record(TestResult::fail(
                "Genesis account funded",
                format!("RPC error: {e}"),
            ));
        }
    }
}

/// Verify blocks are being produced (block number > 0 or progressing).
pub async fn test_block_production(runner: &mut TestRunner) {
    let rpc = runner.primary_rpc();

    match rpc.block_number().await {
        Ok(block) => {
            if block > 0 {
                runner.record(TestResult::pass(
                    "Blocks being produced",
                    format!("current block #{block}"),
                ));
            } else {
                // Wait a bit and check again
                tokio::time::sleep(std::time::Duration::from_secs(6)).await;
                match rpc.block_number().await {
                    Ok(new_block) if new_block > 0 => {
                        runner.record(TestResult::pass(
                            "Blocks being produced",
                            format!("block advanced to #{new_block}"),
                        ));
                    }
                    _ => {
                        runner.record(TestResult::fail(
                            "Blocks being produced",
                            "block number stuck at 0",
                        ));
                    }
                }
            }
        }
        Err(e) => {
            runner.record(TestResult::fail(
                "Blocks being produced",
                format!("RPC error: {e}"),
            ));
        }
    }
}
