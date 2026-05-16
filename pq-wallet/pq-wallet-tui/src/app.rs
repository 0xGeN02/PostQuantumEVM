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
    Blocks,
    Network,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Wallet, Tab::Transactions, Tab::Blocks, Tab::Network];

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Wallet => "Wallet",
            Tab::Transactions => "Transactions",
            Tab::Blocks => "Blocks",
            Tab::Network => "Network",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Tab::Wallet => Tab::Transactions,
            Tab::Transactions => Tab::Blocks,
            Tab::Blocks => Tab::Network,
            Tab::Network => Tab::Wallet,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Tab::Wallet => Tab::Network,
            Tab::Transactions => Tab::Wallet,
            Tab::Blocks => Tab::Transactions,
            Tab::Network => Tab::Blocks,
        }
    }
}

/// A simplified transaction record for display.
#[derive(Debug, Clone)]
pub struct TxRecord {
    pub hash: String,
    pub block: String,
    pub from: String,
    pub to: Option<String>,
    pub value_wei: String,
    /// PQ-specific: signature size in bytes.
    pub sig_size: usize,
    /// Transaction type (0x50 = PQ).
    pub tx_type: String,
    /// Input data (calldata) hex string.
    pub input: String,
    /// Contract address created (if contract creation tx).
    pub contract_address: Option<String>,
}

/// Tx category for display purposes.
#[derive(Debug, Clone, PartialEq)]
pub enum TxKind {
    /// Simple value transfer.
    Transfer,
    /// Contract deployment (to = None).
    Deploy,
    /// Contract call (has calldata).
    ContractCall,
}

impl TxRecord {
    /// Determine the kind of transaction.
    pub fn kind(&self) -> TxKind {
        if self.to.is_none() {
            TxKind::Deploy
        } else if self.input.len() > 2 && self.input != "0x" {
            TxKind::ContractCall
        } else {
            TxKind::Transfer
        }
    }

    /// Get the 4-byte function selector if this is a contract call.
    pub fn function_selector(&self) -> Option<String> {
        if self.kind() == TxKind::ContractCall && self.input.len() >= 10 {
            Some(self.input[..10].to_string())
        } else {
            None
        }
    }

    /// Get calldata size in bytes.
    pub fn calldata_size(&self) -> usize {
        let hex_str = self.input.strip_prefix("0x").unwrap_or(&self.input);
        hex_str.len() / 2
    }
}

/// A block record for the Blocks tab.
#[derive(Debug, Clone)]
pub struct BlockRecord {
    pub number: u64,
    pub hash: String,
    pub timestamp: u64,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub tx_count: usize,
    pub base_fee: u128,
    pub miner: String,
    /// Raw `extraData` hex string from the block header (contains ML-DSA-65 seal).
    pub extra_data: String,
}

/// Interactive action mode (overlay prompts).
#[derive(Debug, Clone, PartialEq)]
pub enum ActionMode {
    /// Normal mode — no action in progress.
    None,
    /// Sending a transfer: collecting fields.
    Send { field: usize, to: String, value: String },
    /// Deploying a contract: collecting init code.
    Deploy { field: usize, code: String, gas_limit: String },
    /// Calling a contract: collecting to + data.
    Call { field: usize, to: String, data: String },
    /// Show result of last action.
    Result { message: String, success: bool },
}

/// Which action triggered the passphrase prompt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PendingActionKind {
    Send,
    Deploy,
}

/// A completed action ready for async execution.
#[derive(Debug, Clone)]
pub enum PendingExec {
    Send { to: String, value: String },
    Deploy { code: String, gas_limit: String },
    Call { to: String, data: String },
}

/// The main application state.
pub struct App {
    /// Currently active tab.
    pub active_tab: Tab,
    /// Whether the app should quit.
    pub should_quit: bool,

    // ─── Action mode (interactive prompts) ───
    pub action: ActionMode,
    /// Passphrase for signing (set once during session).
    pub passphrase: Option<String>,
    /// Passphrase input buffer (when prompting).
    pub passphrase_input: String,
    pub asking_passphrase: bool,
    /// Which action triggered the passphrase prompt (Send or Deploy).
    pub pending_action_kind: Option<PendingActionKind>,
    /// A completed action waiting for async execution.
    pub pending_exec: Option<PendingExec>,

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
    pub tx_table_state: ratatui::widgets::TableState,

    // ─── Blocks ───
    pub blocks: Vec<BlockRecord>,
    pub block_selected: usize,
    pub block_table_state: ratatui::widgets::TableState,
    /// Block range pagination: the highest block number currently displayed.
    /// `None` means "follow chain tip" (latest).
    pub block_page_end: Option<u64>,

    // ─── Search ───
    pub search_mode: bool,
    pub search_input: String,
    pub search_error: Option<String>,

    // ─── Address viewer ───
    pub address_lookup: Option<AddressInfo>,
    pub address_input: String,
    pub showing_address_viewer: bool,

    // ─── Internal ───
    pub rpc: RpcClient,
    pub tick_count: u64,
}

/// Information about a looked-up address.
#[derive(Debug, Clone)]
pub struct AddressInfo {
    pub address: String,
    pub balance_wei: u128,
    pub nonce: u64,
    /// Whether this is a contract (has code). Currently always false
    /// until eth_getCode is added to RpcClient.
    pub is_contract: bool,
}

impl App {
    pub fn new(rpc_url: String, keystore_path: String) -> Self {
        Self {
            active_tab: Tab::Wallet,
            should_quit: false,

            action: ActionMode::None,
            passphrase: None,
            passphrase_input: String::new(),
            asking_passphrase: false,
            pending_action_kind: None,
            pending_exec: None,

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
            tx_table_state: ratatui::widgets::TableState::default(),

            blocks: Vec::new(),
            block_selected: 0,
            block_table_state: ratatui::widgets::TableState::default(),
            block_page_end: None,

            search_mode: false,
            search_input: String::new(),
            search_error: None,

            address_lookup: None,
            address_input: String::new(),
            showing_address_viewer: false,

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
        if self.transactions.is_empty() || self.tick_count.is_multiple_of(3) {
            self.scan_recent_transactions().await;
        }

        // Scan recent blocks for the Blocks tab
        self.scan_recent_blocks().await;
    }

    /// Scan the last N blocks for transactions involving our address.
    async fn scan_recent_transactions(&mut self) {
        let Some(addr) = self.address else { return };
        let addr_lower = format!("{addr:?}").to_lowercase();

        let end_block = self.block_number;
        let start_block = end_block.saturating_sub(200); // last 200 blocks

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
                let tx_type = tx.get("type").and_then(|v| v.as_str()).unwrap_or("0x0").to_string();
                let input = tx.get("input").and_then(|v| v.as_str()).unwrap_or("0x").to_string();

                // For contract creation txs, try to get the receipt for the contract address
                let contract_address = if to.is_none() {
                    // We'll try to fetch the receipt to get the contract address
                    if let Ok(Some(receipt)) = self.rpc.get_transaction_receipt(&hash).await {
                        receipt.contract_address
                    } else {
                        None
                    }
                } else {
                    None
                };

                txs.push(TxRecord {
                    hash,
                    block: block_number_str.clone(),
                    from,
                    to,
                    value_wei: value,
                    sig_size: self.sig_size,
                    tx_type,
                    input,
                    contract_address,
                });
            }

            // Limit to 20 most recent transactions
            if txs.len() >= 20 {
                break;
            }
        }

        self.transactions = txs;
    }

    /// Scan the last N blocks and store their metadata.
    async fn scan_recent_blocks(&mut self) {
        let end_block = self.block_page_end.unwrap_or(self.block_number);
        let start_block = end_block.saturating_sub(29); // 30 blocks per page

        let mut blocks = Vec::new();

        for block_num in (start_block..=end_block).rev() {
            let Ok(Some(block)) = self.rpc.get_block_by_number(block_num).await else {
                continue;
            };

            let number = parse_hex_u64_val(block.get("number"));
            let hash = block.get("hash").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let timestamp = parse_hex_u64_val(block.get("timestamp"));
            let gas_used = parse_hex_u64_val(block.get("gasUsed"));
            let gas_limit = parse_hex_u64_val(block.get("gasLimit"));
            let base_fee = parse_hex_u128_val(block.get("baseFeePerGas"));
            let miner = block.get("miner").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let extra_data = block.get("extraData").and_then(|v| v.as_str()).unwrap_or("0x").to_string();
            let tx_count = block
                .get("transactions")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);

            blocks.push(BlockRecord {
                number,
                hash,
                timestamp,
                gas_used,
                gas_limit,
                tx_count,
                base_fee,
                miner,
                extra_data,
            });
        }

        self.blocks = blocks;
    }

    /// Load the previous page of blocks (older).
    pub fn blocks_page_prev(&mut self) {
        let current_end = self.block_page_end.unwrap_or(self.block_number);
        let new_end = current_end.saturating_sub(30);
        if new_end > 0 {
            self.block_page_end = Some(new_end);
            self.block_selected = 0;
        }
    }

    /// Load the next page of blocks (newer) or jump to latest.
    pub fn blocks_page_next(&mut self) {
        if let Some(end) = self.block_page_end {
            let new_end = end + 30;
            if new_end >= self.block_number {
                // Back to following the chain tip
                self.block_page_end = None;
            } else {
                self.block_page_end = Some(new_end);
            }
            self.block_selected = 0;
        }
        // If already None (following tip), do nothing
    }

    /// Search for a block by number or hash. Returns true if refresh needed.
    pub async fn search_block(&mut self, query: &str) -> bool {
        let query = query.trim();

        // Try as block number first
        if let Ok(num) = query.parse::<u64>() {
            if num <= self.block_number {
                // Navigate to the page containing this block
                // Set page_end so this block is visible
                let page_end = ((num / 30) + 1) * 30;
                self.block_page_end = Some(page_end.min(self.block_number));
                self.active_tab = Tab::Blocks;
                self.search_error = None;
                return true;
            } else {
                self.search_error = Some(format!("Block #{num} does not exist yet (tip: #{})", self.block_number));
                return false;
            }
        }

        // Try as hex block number (0x...)
        if query.starts_with("0x") || query.starts_with("0X") {
            let hex_str = &query[2..];

            // Could be a block hash (32 bytes = 64 hex chars)
            if hex_str.len() == 64 {
                if let Ok(Some(block)) = self.rpc.get_block_by_hash(query).await {
                    let num = parse_hex_u64_val(block.get("number"));
                    let page_end = ((num / 30) + 1) * 30;
                    self.block_page_end = Some(page_end.min(self.block_number));
                    self.active_tab = Tab::Blocks;
                    self.search_error = None;
                    return true;
                } else {
                    self.search_error = Some("Block hash not found".to_string());
                    return false;
                }
            }

            // Try as hex number
            if let Ok(num) = u64::from_str_radix(hex_str, 16) {
                if num <= self.block_number {
                    let page_end = ((num / 30) + 1) * 30;
                    self.block_page_end = Some(page_end.min(self.block_number));
                    self.active_tab = Tab::Blocks;
                    self.search_error = None;
                    return true;
                }
            }
        }

        self.search_error = Some("Invalid search: enter block number or 0x-prefixed hash".to_string());
        false
    }

    /// Look up an address and fetch its info.
    pub async fn lookup_address(&mut self, addr_str: &str) {
        let addr_clean = addr_str.strip_prefix("0x").unwrap_or(addr_str);
        let addr_bytes = match hex::decode(addr_clean) {
            Ok(b) if b.len() == 20 => b,
            _ => {
                self.address_lookup = Some(AddressInfo {
                    address: addr_str.to_string(),
                    balance_wei: 0,
                    nonce: 0,
                    is_contract: false,
                });
                return;
            }
        };

        let addr = Address::from_slice(&addr_bytes);
        let balance = self.rpc.get_balance(addr).await.unwrap_or(0);
        let nonce = self.rpc.get_nonce(addr).await.unwrap_or(0);

        // Check if address has code (is a contract)
        // Note: would need eth_getCode in RpcClient for proper detection.
        let is_contract = false;

        self.address_lookup = Some(AddressInfo {
            address: format!("{addr:?}"),
            balance_wei: balance,
            nonce,
            is_contract,
        });
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

    // ─── Action execution ───

    /// Execute a send/transfer action.
    pub async fn execute_send(&mut self, to: &str, value_str: &str) {
        let Some(passphrase) = &self.passphrase else {
            self.action = ActionMode::Result {
                message: "No passphrase set".to_string(),
                success: false,
            };
            return;
        };

        let path = std::path::Path::new(&self.keystore_path);
        let keypair = match pq_wallet_core::Keystore::load(path, passphrase) {
            Ok(kp) => kp,
            Err(e) => {
                self.action = ActionMode::Result {
                    message: format!("Keystore error: {e}"),
                    success: false,
                };
                return;
            }
        };

        let value: u128 = value_str.parse().unwrap_or(0);
        let to_clean = to.strip_prefix("0x").unwrap_or(to);
        let to_bytes = match hex::decode(to_clean) {
            Ok(b) if b.len() == 20 => b,
            _ => {
                self.action = ActionMode::Result {
                    message: "Invalid address (must be 20 bytes hex)".to_string(),
                    success: false,
                };
                return;
            }
        };
        let to_addr = alloy_primitives::Address::from_slice(&to_bytes);

        let chain_id = self.chain_id;
        let nonce = match self.rpc.get_nonce(keypair.address()).await {
            Ok(n) => n,
            Err(e) => {
                self.action = ActionMode::Result {
                    message: format!("Nonce fetch failed: {e}"),
                    success: false,
                };
                return;
            }
        };
        let gas_price = self.gas_price.max(1_000_000_000); // min 1 Gwei

        let tx = pq_wallet_core::tx::PqTxRequest {
            chain_id,
            nonce,
            to: Some(to_addr),
            value,
            gas_limit: 21000,
            gas_price,
            input: vec![],
        };

        let signer = pq_wallet_core::PqSigner::new(&keypair);
        let signed = signer.sign(tx);
        let raw_hex = format!("0x{}", hex::encode(signed.encode()));

        match self.rpc.send_raw_transaction(&raw_hex).await {
            Ok(hash) => {
                self.action = ActionMode::Result {
                    message: format!("Tx sent! Hash: {hash}"),
                    success: true,
                };
            }
            Err(e) => {
                self.action = ActionMode::Result {
                    message: format!("Broadcast failed: {e}"),
                    success: false,
                };
            }
        }
    }

    /// Execute a contract deployment.
    pub async fn execute_deploy(&mut self, code: &str, gas_limit_str: &str) {
        let Some(passphrase) = &self.passphrase else {
            self.action = ActionMode::Result {
                message: "No passphrase set".to_string(),
                success: false,
            };
            return;
        };

        let path = std::path::Path::new(&self.keystore_path);
        let keypair = match pq_wallet_core::Keystore::load(path, passphrase) {
            Ok(kp) => kp,
            Err(e) => {
                self.action = ActionMode::Result {
                    message: format!("Keystore error: {e}"),
                    success: false,
                };
                return;
            }
        };

        let code_clean = code.strip_prefix("0x").unwrap_or(code);
        let input = match hex::decode(code_clean) {
            Ok(b) => b,
            Err(_) => {
                self.action = ActionMode::Result {
                    message: "Invalid hex in contract code".to_string(),
                    success: false,
                };
                return;
            }
        };

        let gas_limit: u64 = gas_limit_str.parse().unwrap_or(1_000_000);
        let chain_id = self.chain_id;
        let nonce = match self.rpc.get_nonce(keypair.address()).await {
            Ok(n) => n,
            Err(e) => {
                self.action = ActionMode::Result {
                    message: format!("Nonce fetch failed: {e}"),
                    success: false,
                };
                return;
            }
        };
        let gas_price = self.gas_price.max(1_000_000_000);

        let tx = pq_wallet_core::tx::PqTxRequest {
            chain_id,
            nonce,
            to: None,
            value: 0,
            gas_limit,
            gas_price,
            input,
        };

        let signer = pq_wallet_core::PqSigner::new(&keypair);
        let signed = signer.sign(tx);
        let raw_hex = format!("0x{}", hex::encode(signed.encode()));

        match self.rpc.send_raw_transaction(&raw_hex).await {
            Ok(hash) => {
                self.action = ActionMode::Result {
                    message: format!("Deploy tx sent! Hash: {hash}"),
                    success: true,
                };
            }
            Err(e) => {
                self.action = ActionMode::Result {
                    message: format!("Deploy failed: {e}"),
                    success: false,
                };
            }
        }
    }

    /// Execute a read-only contract call.
    pub async fn execute_call(&mut self, to: &str, data: &str) {
        let to_formatted = format!("0x{}", to.strip_prefix("0x").unwrap_or(to));
        let data_formatted = format!("0x{}", data.strip_prefix("0x").unwrap_or(data));
        let from = self.address.map(|a| format!("{a:?}"));

        match self.rpc.eth_call(from.as_deref(), &to_formatted, &data_formatted).await {
            Ok(result) => {
                self.action = ActionMode::Result {
                    message: format!("Result: {result}"),
                    success: true,
                };
            }
            Err(e) => {
                self.action = ActionMode::Result {
                    message: format!("Call failed: {e}"),
                    success: false,
                };
            }
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Parse a hex u64 from a JSON Value (e.g. "0x1a").
fn parse_hex_u64_val(v: Option<&serde_json::Value>) -> u64 {
    v.and_then(|v| v.as_str())
        .map(|s| {
            let s = s.strip_prefix("0x").unwrap_or(s);
            u64::from_str_radix(s, 16).unwrap_or(0)
        })
        .unwrap_or(0)
}

/// Parse a hex u128 from a JSON Value (e.g. "0x34a360cb").
fn parse_hex_u128_val(v: Option<&serde_json::Value>) -> u128 {
    v.and_then(|v| v.as_str())
        .map(|s| {
            let s = s.strip_prefix("0x").unwrap_or(s);
            u128::from_str_radix(s, 16).unwrap_or(0)
        })
        .unwrap_or(0)
}
