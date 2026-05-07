//! Application state for the PQ wallet TUI.

use alloy_primitives::Address;
use pq_wallet_core::RpcClient;
use sha3::{Shake256, digest::{Update, ExtendableOutput, XofReader}};
use tiny_keccak::{Hasher, Keccak};

/// Active tab in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Wallet,
    Transactions,
    Network,
}

impl Tab {
    pub const ALL: [Tab; 3] = [Tab::Wallet, Tab::Transactions, Tab::Network];

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Wallet => "Wallet",
            Tab::Transactions => "Transactions",
            Tab::Network => "Network",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Tab::Wallet => Tab::Transactions,
            Tab::Transactions => Tab::Network,
            Tab::Network => Tab::Wallet,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Tab::Wallet => Tab::Network,
            Tab::Transactions => Tab::Wallet,
            Tab::Network => Tab::Transactions,
        }
    }
}

/// A simplified transaction record for display.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TxRecord {
    pub hash: String,
    pub block: String,
    pub from: String,
    pub to: Option<String>,
    pub value_wei: String,
    pub gas_used: String,
    pub status: String,
    /// PQ-specific: signature size in bytes.
    pub sig_size: usize,
    /// PQ-specific: public key size in bytes.
    pub pk_size: usize,
    /// Transaction type (0x50 = PQ).
    pub tx_type: String,
}

/// The main application state.
pub struct App {
    /// Currently active tab.
    pub active_tab: Tab,
    /// Whether the app should quit.
    pub should_quit: bool,

    // ─── Wallet info ───
    pub address: Option<Address>,
    pub keystore_path: String,
    pub algorithm: &'static str,
    pub pk_size: usize,
    pub sig_size: usize,

    // ─── Hash comparison (computed from public key) ───
    /// Full SHAKE-256 hash of the public key (32 bytes, hex).
    pub shake256_hash: Option<String>,
    /// Full keccak256 hash of the public key (32 bytes, hex).
    pub keccak256_hash: Option<String>,
    /// Address derived via SHAKE-256 (our PQ method).
    pub addr_shake256: Option<String>,
    /// Address that would be derived via keccak256 (classical method).
    pub addr_keccak256: Option<String>,

    // ─── Balance ───
    pub balance_wei: u128,

    // ─── Network info ───
    pub chain_id: u64,
    pub block_number: u64,
    pub gas_price: u128,
    pub rpc_url: String,
    pub connected: bool,

    // ─── Transactions ───
    pub transactions: Vec<TxRecord>,
    pub tx_selected: usize,

    // ─── Internal ───
    pub rpc: RpcClient,
    pub tick_count: u64,
}

impl App {
    pub fn new(rpc_url: String, keystore_path: String) -> Self {
        Self {
            active_tab: Tab::Wallet,
            should_quit: false,

            address: None,
            keystore_path,
            algorithm: "ML-DSA-65 (CRYSTALS-Dilithium)",
            pk_size: 1952,
            sig_size: 3309,

            shake256_hash: None,
            keccak256_hash: None,
            addr_shake256: None,
            addr_keccak256: None,

            balance_wei: 0,

            chain_id: 0,
            block_number: 0,
            gas_price: 0,
            rpc_url: rpc_url.clone(),
            connected: false,

            transactions: Vec::new(),
            tx_selected: 0,

            rpc: RpcClient::new(&rpc_url),
            tick_count: 0,
        }
    }

    /// Fetch network and balance data from the node.
    pub async fn refresh(&mut self) {
        self.tick_count += 1;

        // Chain ID
        if let Ok(chain_id) = self.rpc.chain_id().await {
            self.chain_id = chain_id;
            self.connected = true;
        } else {
            self.connected = false;
            return;
        }

        // Gas price
        if let Ok(gp) = self.rpc.gas_price().await {
            self.gas_price = gp;
        }

        // Balance
        if let Some(addr) = self.address {
            if let Ok(bal) = self.rpc.get_balance(addr).await {
                self.balance_wei = bal;
            }
        }

        // Block number
        if let Ok(block) = self.rpc.block_number().await {
            self.block_number = block;
        }

        // Scan recent blocks for transactions (every 3rd tick or first time)
        if self.transactions.is_empty() || self.tick_count % 3 == 0 {
            self.scan_recent_transactions().await;
        }
    }

    /// Scan the last N blocks for transactions involving our address.
    async fn scan_recent_transactions(&mut self) {
        let Some(addr) = self.address else { return };
        let addr_lower = format!("{addr:?}").to_lowercase();

        let end_block = self.block_number;
        let start_block = end_block.saturating_sub(50); // last 50 blocks

        let mut txs = Vec::new();

        for block_num in (start_block..=end_block).rev() {
            let Ok(Some(block)) = self.rpc.get_block_by_number(block_num).await else {
                continue;
            };

            let block_number_str = block
                .get("number")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();

            let Some(transactions) = block.get("transactions").and_then(|v| v.as_array()) else {
                continue;
            };

            for tx in transactions {
                let from = tx.get("from").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                let to = tx.get("to").and_then(|v| v.as_str()).map(|s| s.to_lowercase());

                let is_ours = from == addr_lower
                    || to.as_deref() == Some(&addr_lower);

                if !is_ours {
                    continue;
                }

                let hash = tx.get("hash").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let value = tx.get("value").and_then(|v| v.as_str()).unwrap_or("0x0").to_string();
                let gas = tx.get("gas").and_then(|v| v.as_str()).unwrap_or("0x0").to_string();
                let tx_type = tx.get("type").and_then(|v| v.as_str()).unwrap_or("0x0").to_string();

                txs.push(TxRecord {
                    hash,
                    block: block_number_str.clone(),
                    from,
                    to,
                    value_wei: value,
                    gas_used: gas,
                    status: "0x1".to_string(), // assume success if in block
                    sig_size: self.sig_size,
                    pk_size: self.pk_size,
                    tx_type,
                });
            }

            // Limit to 20 most recent transactions
            if txs.len() >= 20 {
                break;
            }
        }

        self.transactions = txs;
    }

    /// Load wallet address from keystore (no passphrase needed).
    pub fn load_keystore(&mut self) {
        let path = std::path::Path::new(&self.keystore_path);

        // Load address
        if let Ok(addr_str) = pq_wallet_core::Keystore::address_from_file(path) {
            let clean = addr_str.strip_prefix("0x").unwrap_or(&addr_str);
            if let Ok(bytes) = hex::decode(clean) {
                if bytes.len() == 20 {
                    self.address = Some(Address::from_slice(&bytes));
                }
            }
        }

        // Load public key and compute hashes
        if let Ok(pk_hex) = pq_wallet_core::Keystore::public_key_from_file(path) {
            if let Ok(pk_bytes) = hex::decode(&pk_hex) {
                // SHAKE-256 (our PQ method)
                let mut shake = Shake256::default();
                shake.update(&pk_bytes);
                let mut shake_out = [0u8; 32];
                shake.finalize_xof().read(&mut shake_out);
                self.shake256_hash = Some(format!("0x{}", hex::encode(shake_out)));
                self.addr_shake256 = Some(format!("0x{}", hex::encode(&shake_out[12..])));

                // keccak256 (classical Ethereum method)
                let mut keccak = Keccak::v256();
                keccak.update(&pk_bytes);
                let mut keccak_out = [0u8; 32];
                keccak.finalize(&mut keccak_out);
                self.keccak256_hash = Some(format!("0x{}", hex::encode(keccak_out)));
                self.addr_keccak256 = Some(format!("0x{}", hex::encode(&keccak_out[12..])));
            }
        }
    }

    /// Navigate to next tab.
    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
    }

    /// Navigate to previous tab.
    pub fn prev_tab(&mut self) {
        self.active_tab = self.active_tab.prev();
    }

    /// Format balance as qETH.
    pub fn balance_qeth(&self) -> String {
        let eth = self.balance_wei as f64 / 1e18;
        format!("{eth:.6} qETH")
    }

    /// Format balance in wei.
    pub fn balance_wei_str(&self) -> String {
        format!("{} wei", self.balance_wei)
    }
}
