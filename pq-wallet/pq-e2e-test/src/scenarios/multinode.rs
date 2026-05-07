//! Multi-node consistency validation scenarios.
//!
//! These tests verify that all PoA validators agree on chain state.

use super::runner::{TestResult, TestRunner};

/// Verify all nodes report the same chain ID.
pub async fn test_chain_id_consistency(runner: &mut TestRunner) {
    let rpcs = runner.all_rpcs();
    let mut chain_ids: Vec<(String, u64)> = Vec::new();

    for (i, rpc) in rpcs.iter().enumerate() {
        match rpc.chain_id().await {
            Ok(cid) => chain_ids.push((runner.rpc_endpoints[i].clone(), cid)),
            Err(e) => {
                runner.record(TestResult::fail(
                    "Chain ID consistent across nodes",
                    format!("node {} ({}) error: {e}", i + 1, runner.rpc_endpoints[i]),
                ));
                return;
            }
        }
    }

    let first_cid = chain_ids[0].1;
    let all_same = chain_ids.iter().all(|(_, cid)| *cid == first_cid);

    if all_same {
        runner.record(TestResult::pass(
            "Chain ID consistent across nodes",
            format!("all {} nodes report chain_id={first_cid}", chain_ids.len()),
        ));
    } else {
        runner.record(TestResult::fail(
            "Chain ID consistent across nodes",
            format!("mismatch: {:?}", chain_ids),
        ));
    }
}

/// Verify all nodes are at approximately the same block height.
pub async fn test_block_height_consistency(runner: &mut TestRunner) {
    let rpcs = runner.all_rpcs();
    let mut heights: Vec<(String, u64)> = Vec::new();

    for (i, rpc) in rpcs.iter().enumerate() {
        match rpc.block_number().await {
            Ok(h) => heights.push((runner.rpc_endpoints[i].clone(), h)),
            Err(e) => {
                runner.record(TestResult::fail(
                    "Block height consistent (±2)",
                    format!("node {} error: {e}", i + 1),
                ));
                return;
            }
        }
    }

    let max_h = heights.iter().map(|(_, h)| *h).max().unwrap_or(0);
    let min_h = heights.iter().map(|(_, h)| *h).min().unwrap_or(0);
    let drift = max_h - min_h;

    if drift <= 2 {
        runner.record(TestResult::pass(
            "Block height consistent (±2)",
            format!(
                "heights: {:?}, drift={drift} blocks",
                heights.iter().map(|(_, h)| *h).collect::<Vec<_>>()
            ),
        ));
    } else {
        runner.record(TestResult::fail(
            "Block height consistent (±2)",
            format!(
                "drift={drift} blocks (max allowed: 2): {:?}",
                heights
            ),
        ));
    }
}

/// Verify all nodes report the same balance for a genesis account.
pub async fn test_state_consistency(runner: &mut TestRunner) {
    let rpcs = runner.all_rpcs();
    let test_addr_str = "f39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
    let addr = alloy_primitives::Address::from_slice(&hex::decode(test_addr_str).unwrap());

    let mut balances: Vec<(String, u128)> = Vec::new();

    for (i, rpc) in rpcs.iter().enumerate() {
        match rpc.get_balance(addr).await {
            Ok(b) => balances.push((runner.rpc_endpoints[i].clone(), b)),
            Err(e) => {
                runner.record(TestResult::fail(
                    "State consistent across nodes",
                    format!("node {} error: {e}", i + 1),
                ));
                return;
            }
        }
    }

    let first_bal = balances[0].1;
    let all_same = balances.iter().all(|(_, b)| *b == first_bal);

    if all_same {
        let qeth = first_bal as f64 / 1e18;
        runner.record(TestResult::pass(
            "State consistent across nodes",
            format!(
                "all {} nodes agree: balance={qeth:.4} qETH for test account",
                balances.len()
            ),
        ));
    } else {
        runner.record(TestResult::fail(
            "State consistent across nodes",
            format!("balance mismatch: {:?}", balances),
        ));
    }
}
