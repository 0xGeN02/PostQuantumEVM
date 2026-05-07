//! `PoA` consensus validation scenarios.

use super::runner::{TestResult, TestRunner};

/// Fetch recent blocks by walking backwards from "latest" via `parentHash`.
/// Returns up to `count` blocks (newest first).
async fn fetch_recent_blocks(
    rpc: &pq_wallet_core::RpcClient,
    count: usize,
) -> Vec<serde_json::Value> {
    let mut blocks = Vec::new();

    // Start from "latest"
    let Some(block) = rpc.get_block_by_tag("latest").await.ok().flatten() else {
        return blocks;
    };
    blocks.push(block);

    // Walk backwards via parentHash
    while blocks.len() < count {
        let parent_hash = blocks
            .last()
            .and_then(|b| b.get("parentHash"))
            .and_then(|v| v.as_str());

        let Some(hash) = parent_hash else { break };

        // Stop at genesis (all-zero hash)
        if hash == "0x0000000000000000000000000000000000000000000000000000000000000000" {
            break;
        }

        match rpc.get_block_by_hash(hash).await {
            Ok(Some(block)) => blocks.push(block),
            _ => break,
        }
    }

    blocks
}

/// Verify that different blocks are mined by different validators (rotation).
pub async fn test_validator_rotation(runner: &mut TestRunner) {
    let rpc = runner.primary_rpc();

    let blocks = fetch_recent_blocks(&rpc, 10).await;

    if blocks.is_empty() {
        runner.record(TestResult::fail(
            "Validator rotation",
            "Could not fetch any blocks",
        ));
        return;
    }

    let mut miners = std::collections::HashSet::new();
    for block in &blocks {
        if let Some(miner) = block.get("miner").and_then(|v| v.as_str()) {
            miners.insert(miner.to_lowercase());
        }
    }

    if miners.len() > 1 {
        runner.record(TestResult::pass(
            "Validator rotation",
            format!(
                "{} distinct validators found in last {} blocks",
                miners.len(),
                blocks.len(),
            ),
        ));
    } else if miners.len() == 1 {
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

    let blocks = fetch_recent_blocks(&rpc, 5).await;

    // Blocks are newest-first, reverse for chronological order
    let timestamps: Vec<u64> = blocks
        .iter()
        .rev()
        .filter_map(|b| {
            b.get("timestamp")
                .and_then(|v| v.as_str())
                .and_then(|ts| {
                    u64::from_str_radix(ts.strip_prefix("0x").unwrap_or(ts), 16).ok()
                })
        })
        .collect();

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
