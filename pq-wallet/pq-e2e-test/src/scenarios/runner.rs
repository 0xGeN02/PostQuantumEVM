//! Test runner orchestrating all E2E scenarios.

use std::time::Duration;

use anyhow::{bail, Result};
use pq_wallet_core::RpcClient;

use super::{chain, consensus, contracts, fees, multinode, transactions};

/// Outcome of a single test scenario.
#[derive(Debug)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

impl TestResult {
    pub fn pass(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { name: name.into(), passed: true, detail: detail.into() }
    }

    pub fn fail(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { name: name.into(), passed: false, detail: detail.into() }
    }
}

/// The test runner holds configuration and executes scenarios in order.
pub struct TestRunner {
    pub rpc_endpoints: Vec<String>,
    pub keystore_path: Option<String>,
    pub passphrase: String,
    pub readonly: bool,
    pub verbose: bool,
    pub wait_secs: u64,
    pub results: Vec<TestResult>,
    /// Hash of the last sent transaction (shared between test phases).
    pub last_tx_hash: Option<String>,
}

impl TestRunner {
    pub fn new(
        rpc_endpoints: Vec<String>,
        keystore_path: Option<String>,
        passphrase: String,
        readonly: bool,
        verbose: bool,
        wait_secs: u64,
    ) -> Self {
        Self {
            rpc_endpoints,
            keystore_path,
            passphrase,
            readonly,
            verbose,
            wait_secs,
            results: Vec::new(),
            last_tx_hash: None,
        }
    }

    /// Primary RPC client (first endpoint).
    pub fn primary_rpc(&self) -> RpcClient {
        RpcClient::new(&self.rpc_endpoints[0])
    }

    /// All RPC clients.
    pub fn all_rpcs(&self) -> Vec<RpcClient> {
        self.rpc_endpoints.iter().map(RpcClient::new).collect()
    }

    /// Run all test scenarios.
    pub async fn run(&mut self) -> Result<()> {
        // Wait for node availability
        self.wait_for_node().await?;

        // Phase 1: Chain identity (read-only)
        self.section("Chain Identity");
        chain::test_chain_id(self).await;
        chain::test_genesis_accounts(self).await;
        chain::test_block_production(self).await;

        // Phase 2: Consensus validation (read-only)
        self.section("PoA Consensus");
        consensus::test_validator_rotation(self).await;
        consensus::test_block_timestamps(self).await;

        // Phase 3: Fee model (read-only)
        self.section("EIP-1559 Fee Model");
        fees::test_base_fee_exists(self).await;
        fees::test_gas_price(self).await;

        // Phase 4: Transactions (requires keystore)
        if !self.readonly {
            if self.keystore_path.is_some() {
                self.section("PQ Transactions");
                transactions::test_send_transfer(self).await;
                transactions::test_receipt_validation(self).await;
                transactions::test_nonce_increment(self).await;

                // Phase 5: Contracts
                self.section("Smart Contracts");
                contracts::test_deploy_contract(self).await;
                contracts::test_call_contract(self).await;
            } else {
                println!("  ⚠ Skipping transaction tests (no --keystore provided)");
                println!();
            }
        } else {
            println!("  ⚠ Skipping transaction tests (--readonly mode)");
            println!();
        }

        // Phase 6: Multi-node consistency
        if self.rpc_endpoints.len() > 1 {
            self.section("Multi-Node Consistency");
            multinode::test_chain_id_consistency(self).await;
            multinode::test_block_height_consistency(self).await;
            multinode::test_state_consistency(self).await;
        }

        // Report results
        self.report()?;

        Ok(())
    }

    /// Wait for the primary node to become responsive.
    async fn wait_for_node(&self) -> Result<()> {
        print!("  Waiting for node at {} ", self.rpc_endpoints[0]);
        let rpc = self.primary_rpc();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(self.wait_secs);

        loop {
            if rpc.chain_id().await.is_ok() {
                println!("✓");
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                println!("✗");
                bail!(
                    "Node at {} not available after {}s",
                    self.rpc_endpoints[0],
                    self.wait_secs
                );
            }
            print!(".");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    /// Record a test result and print status.
    pub fn record(&mut self, result: TestResult) {
        let icon = if result.passed { "✓" } else { "✗" };
        let color = if result.passed { "\x1b[32m" } else { "\x1b[31m" };
        let reset = "\x1b[0m";

        println!("  {color}{icon}{reset} {}", result.name);
        if self.verbose || !result.passed {
            println!("    └─ {}", result.detail);
        }

        self.results.push(result);
    }

    /// Print a section header.
    fn section(&self, name: &str) {
        println!("─── {name} ───────────────────────────────────────────");
        println!();
    }

    /// Print final report and return error if any test failed.
    fn report(&self) -> Result<()> {
        let total = self.results.len();
        let passed = self.results.iter().filter(|r| r.passed).count();
        let failed = total - passed;

        println!();
        println!("─── Results ───────────────────────────────────────────");
        println!();
        println!(
            "  Total: {}  Passed: \x1b[32m{}\x1b[0m  Failed: \x1b[31m{}\x1b[0m",
            total, passed, failed
        );

        if failed > 0 {
            println!();
            println!("  Failed tests:");
            for r in self.results.iter().filter(|r| !r.passed) {
                println!("    \x1b[31m✗\x1b[0m {} — {}", r.name, r.detail);
            }
            bail!("{} test(s) failed", failed);
        }

        Ok(())
    }
}
