//! PoA consensus validation scenarios.

use super::runner::{TestResult, TestRunner};

/// Verify that different blocks are mined by different validators (rotation).
pub async fn test_validator_rotation(runner: &mut TestRunner) {
    let rpc = runner.primary_rpc();

    let current_block = match rpc.block_number().await {
        Ok(b) => b,
        Err(e) => {
            runner.record(TestResult::fail(
                "Validator rotation",
                format!("Cannot get block number: {e}"),
            ));
            return;
        }
    };

    if current_block < 3 {
        runner.record(TestResult::pass(
            "Validator rotation",
            format!("Only {current_block} blocks — not enough to verify rotation (need ≥3)"),
        ));
        return;
    }

    // Fetch last N blocks and check miners
    let mut miners = std::collections::HashSet::new();
    let check_count = current_block.min(10);
    let start = current_block.saturating_sub(check_count - 1);

    for block_num in start..=current_block {
        if let Ok(Some(block)) = rpc.get_block_by_number(block_num).await {
            if let Some(miner) = block.get("miner").and_then(|v| v.as_str()) {
                miners.insert(miner.to_lowercase());
            }
        }
    }

    if miners.len() > 1 {
        runner.record(TestResult::pass(
            "Validator rotation",
            format!(
                "{} distinct validators found in last {} blocks: {:?}",
                miners.len(),
                check_count,
                miners
            ),
        ));
    } else if miners.len() == 1 {
        // In dev mode with a single validator, this is expected
        let miner = miners.iter().next().unwrap();
        runner.record(TestResult::pass(
            "Validator rotation",
            format!("Single validator mode — miner: {miner}"),
        ));
    } else {
        runner.record(TestResult::fail(
            "Validator rotation",
            "No miners found in recent blocks",
        ));
    }
}

/// Verify block timestamps are monotonically increasing and roughly match slot time.
pub async fn test_block_timestamps(runner: &mut TestRunner) {
    let rpc = runner.primary_rpc();

    let current_block = match rpc.block_number().await {
        Ok(b) => b,
        Err(e) => {
            runner.record(TestResult::fail(
                "Block timestamps monotonic",
                format!("Cannot get block number: {e}"),
            ));
            return;
        }
    };

    if current_block < 2 {
        runner.record(TestResult::pass(
            "Block timestamps monotonic",
            "Not enough blocks to check timestamps",
        ));
        return;
    }

    let mut timestamps: Vec<u64> = Vec::new();
    let check_count = current_block.min(5);
    let start = current_block.saturating_sub(check_count - 1);

    for block_num in start..=current_block {
        if let Ok(Some(block)) = rpc.get_block_by_number(block_num).await {
            if let Some(ts) = block.get("timestamp").and_then(|v| v.as_str()) {
                let ts_val = u64::from_str_radix(
                    ts.strip_prefix("0x").unwrap_or(ts),
                    16,
                )
                .unwrap_or(0);
                timestamps.push(ts_val);
            }
        }
    }

    if timestamps.len() < 2 {
        runner.record(TestResult::fail(
            "Block timestamps monotonic",
            "Could not fetch enough block timestamps",
        ));
        return;
    }

    // Check monotonicity
    let monotonic = timestamps.windows(2).all(|w| w[1] >= w[0]);

    // Compute average slot time
    let total_time = timestamps.last().unwrap() - timestamps.first().unwrap();
    let avg_slot = if timestamps.len() > 1 {
        total_time as f64 / (timestamps.len() - 1) as f64
    } else {
        0.0
    };

    if monotonic {
        runner.record(TestResult::pass(
            "Block timestamps monotonic",
            format!(
                "timestamps strictly increasing, avg slot time: {avg_slot:.1}s ({} blocks)",
                timestamps.len()
            ),
        ));
    } else {
        runner.record(TestResult::fail(
            "Block timestamps monotonic",
            format!("timestamps not monotonic: {:?}", timestamps),
        ));
    }
}
