//! EIP-1559 fee model validation scenarios.

use super::runner::{TestResult, TestRunner};

/// Verify that blocks have a baseFeePerGas field (EIP-1559 active).
pub async fn test_base_fee_exists(runner: &mut TestRunner) {
    let rpc = runner.primary_rpc();

    let block_num = match rpc.block_number().await {
        Ok(b) => b,
        Err(e) => {
            runner.record(TestResult::fail(
                "EIP-1559 base fee present",
                format!("Cannot get block number: {e}"),
            ));
            return;
        }
    };

    match rpc.get_block_by_number(block_num).await {
        Ok(Some(block)) => {
            if let Some(base_fee_str) = block.get("baseFeePerGas").and_then(|v| v.as_str()) {
                let base_fee = u128::from_str_radix(
                    base_fee_str.strip_prefix("0x").unwrap_or(base_fee_str),
                    16,
                )
                .unwrap_or(0);
                let gwei = base_fee as f64 / 1e9;
                runner.record(TestResult::pass(
                    "EIP-1559 base fee present",
                    format!("block #{block_num} baseFee={gwei:.4} Gwei ({base_fee} wei)"),
                ));
            } else {
                runner.record(TestResult::fail(
                    "EIP-1559 base fee present",
                    format!("block #{block_num} missing baseFeePerGas field"),
                ));
            }
        }
        Ok(None) => {
            runner.record(TestResult::fail(
                "EIP-1559 base fee present",
                format!("block #{block_num} not found"),
            ));
        }
        Err(e) => {
            runner.record(TestResult::fail(
                "EIP-1559 base fee present",
                format!("RPC error: {e}"),
            ));
        }
    }
}

/// Verify eth_gasPrice returns a reasonable value.
pub async fn test_gas_price(runner: &mut TestRunner) {
    let rpc = runner.primary_rpc();

    match rpc.gas_price().await {
        Ok(gas_price) => {
            let gwei = gas_price as f64 / 1e9;
            if gas_price > 0 {
                runner.record(TestResult::pass(
                    "Gas price > 0",
                    format!("gasPrice={gwei:.4} Gwei ({gas_price} wei)"),
                ));
            } else {
                runner.record(TestResult::fail(
                    "Gas price > 0",
                    "gasPrice is 0 — EIP-1559 may not be active",
                ));
            }
        }
        Err(e) => {
            runner.record(TestResult::fail(
                "Gas price > 0",
                format!("RPC error: {e}"),
            ));
        }
    }
}
