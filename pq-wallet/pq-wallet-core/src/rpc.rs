//! Minimal JSON-RPC 2.0 client for Ethereum nodes.
//!
//! Supports:
//! - `eth_getBalance`
//! - `eth_getTransactionCount` (nonce)
//! - `eth_sendRawTransaction`
//! - `eth_getTransactionReceipt`
//! - `eth_chainId`
//! - `eth_gasPrice`
//! - `eth_call`

use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::WalletError;

// ─── JSON-RPC types ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'static str,
    method: &'a str,
    params: Value,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct RpcResponse {
    /// The result value.  We use `Value` (not `Option<Value>`) because serde
    /// deserializes JSON `null` into `None` for `Option<Value>`, making it
    /// impossible to distinguish "field absent" from "result: null".
    /// With `#[serde(default)]` a missing field becomes `Value::Null`, and
    /// `"result": null` also becomes `Value::Null` — callers that accept
    /// nullable results (receipts, blocks) already check `is_null()`.
    #[serde(default)]
    result: Value,
    error: Option<RpcErrorObj>,
}

#[derive(Debug, Deserialize)]
struct RpcErrorObj {
    code: i64,
    message: String,
}

// ─── RpcClient ───────────────────────────────────────────────────────────────

/// A minimal async JSON-RPC 2.0 client.
pub struct RpcClient {
    url: String,
    client: reqwest::Client,
}

impl RpcClient {
    /// Create a new client pointing to `url` (e.g. `http://localhost:8545`).
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into(), client: reqwest::Client::new() }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Get the ETH balance of `address` in wei (as a decimal string).
    pub async fn get_balance(&self, address: Address) -> Result<u128, WalletError> {
        let hex_addr = format!("{address:?}");
        let result = self.call("eth_getBalance", json!([hex_addr, "latest"])).await?;
        parse_hex_u128(&result)
    }

    /// Get the next nonce for `address`.
    pub async fn get_nonce(&self, address: Address) -> Result<u64, WalletError> {
        let hex_addr = format!("{address:?}");
        let result = self.call("eth_getTransactionCount", json!([hex_addr, "latest"])).await?;
        parse_hex_u64(&result)
    }

    /// Get the current chain ID.
    pub async fn chain_id(&self) -> Result<u64, WalletError> {
        let result = self.call("eth_chainId", json!([])).await?;
        parse_hex_u64(&result)
    }

    /// Get the current gas price in wei.
    pub async fn gas_price(&self) -> Result<u128, WalletError> {
        let result = self.call("eth_gasPrice", json!([])).await?;
        parse_hex_u128(&result)
    }

    /// Send a raw signed transaction.
    ///
    /// `raw_tx` should be the hex-encoded bytes (with `0x` prefix).
    /// Returns the transaction hash.
    pub async fn send_raw_transaction(&self, raw_tx: &str) -> Result<String, WalletError> {
        let result = self.call("eth_sendRawTransaction", json!([raw_tx])).await?;
        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| WalletError::RpcParse("expected string tx hash".into()))
    }

    /// Get the transaction receipt for a given tx hash.
    ///
    /// Returns `None` if the transaction is still pending.
    pub async fn get_transaction_receipt(&self, tx_hash: &str) -> Result<Option<TxReceipt>, WalletError> {
        let result = self.call("eth_getTransactionReceipt", json!([tx_hash])).await?;
        if result.is_null() {
            return Ok(None);
        }
        let receipt: TxReceipt = serde_json::from_value(result)
            .map_err(|e| WalletError::RpcParse(format!("receipt parse error: {e}")))?;
        Ok(Some(receipt))
    }

    /// Execute a read-only contract call (`eth_call`).
    ///
    /// This does not create a transaction — it simulates execution against
    /// the latest state and returns the raw output bytes (hex-encoded).
    ///
    /// # Arguments
    ///
    /// * `from` - Optional sender address (for msg.sender context)
    /// * `to` - Contract address to call
    /// * `data` - ABI-encoded function call data (hex with 0x prefix)
    pub async fn eth_call(
        &self,
        from: Option<&str>,
        to: &str,
        data: &str,
    ) -> Result<String, WalletError> {
        let mut call_obj = json!({
            "to": to,
            "data": data,
        });

        if let Some(from_addr) = from {
            call_obj["from"] = json!(from_addr);
        }

        let result = self.call("eth_call", json!([call_obj, "latest"])).await?;
        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| WalletError::RpcParse("expected hex string from eth_call".into()))
    }

    /// Get the current block number.
    pub async fn block_number(&self) -> Result<u64, WalletError> {
        let result = self.call("eth_blockNumber", json!([])).await?;
        parse_hex_u64(&result)
    }

    /// Get a block by number (with full transaction objects).
    ///
    /// Returns the raw JSON value of the block, or None if not found.
    pub async fn get_block_by_number(&self, block: u64) -> Result<Option<Value>, WalletError> {
        let block_hex = format!("0x{block:x}");
        let result = self.call("eth_getBlockByNumber", json!([block_hex, true])).await?;
        if result.is_null() {
            return Ok(None);
        }
        Ok(Some(result))
    }

    /// Get a block by tag (`"latest"`, `"earliest"`, `"pending"`).
    pub async fn get_block_by_tag(&self, tag: &str) -> Result<Option<Value>, WalletError> {
        let result = self.call("eth_getBlockByNumber", json!([tag, true])).await?;
        if result.is_null() {
            return Ok(None);
        }
        Ok(Some(result))
    }

    /// Get a block by hash (with full transaction objects).
    pub async fn get_block_by_hash(&self, hash: &str) -> Result<Option<Value>, WalletError> {
        let result = self.call("eth_getBlockByHash", json!([hash, true])).await?;
        if result.is_null() {
            return Ok(None);
        }
        Ok(Some(result))
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    async fn call(&self, method: &str, params: Value) -> Result<Value, WalletError> {
        let req = RpcRequest { jsonrpc: "2.0", method, params, id: 1 };
        let resp: RpcResponse = self.client.post(&self.url).json(&req).send().await?.json().await?;

        if let Some(err) = resp.error {
            return Err(WalletError::RpcError { code: err.code, message: err.message });
        }

        Ok(resp.result)
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn parse_hex_u128(v: &Value) -> Result<u128, WalletError> {
    let s = v.as_str().ok_or_else(|| WalletError::RpcParse("expected hex string".into()))?;
    let s = s.strip_prefix("0x").unwrap_or(s);
    u128::from_str_radix(s, 16)
        .map_err(|e| WalletError::RpcParse(format!("u128 parse error: {e}")))
}

fn parse_hex_u64(v: &Value) -> Result<u64, WalletError> {
    let s = v.as_str().ok_or_else(|| WalletError::RpcParse("expected hex string".into()))?;
    let s = s.strip_prefix("0x").unwrap_or(s);
    u64::from_str_radix(s, 16)
        .map_err(|e| WalletError::RpcParse(format!("u64 parse error: {e}")))
}

// ─── Receipt type ────────────────────────────────────────────────────────────

/// Transaction receipt returned by `eth_getTransactionReceipt`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxReceipt {
    /// Transaction hash.
    pub transaction_hash: String,
    /// Block hash containing this tx.
    pub block_hash: String,
    /// Block number (hex).
    pub block_number: String,
    /// Transaction index in the block (hex).
    pub transaction_index: String,
    /// Sender address.
    pub from: String,
    /// Recipient address (null for contract creation).
    pub to: Option<String>,
    /// Contract address if this was a contract creation.
    pub contract_address: Option<String>,
    /// Cumulative gas used in the block up to this tx (hex).
    pub cumulative_gas_used: String,
    /// Gas used by this tx (hex).
    pub gas_used: String,
    /// Status: "0x1" = success, "0x0" = revert.
    pub status: String,
}
