//! Minimal JSON-RPC 2.0 client for Ethereum nodes.
//!
//! Supports:
//! - `eth_getBalance`
//! - `eth_getTransactionCount` (nonce)
//! - `eth_sendRawTransaction`
//! - `eth_chainId`
//! - `eth_gasPrice`

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
    result: Option<Value>,
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

    // ── Internal ──────────────────────────────────────────────────────────────

    async fn call(&self, method: &str, params: Value) -> Result<Value, WalletError> {
        let req = RpcRequest { jsonrpc: "2.0", method, params, id: 1 };
        let resp: RpcResponse = self.client.post(&self.url).json(&req).send().await?.json().await?;

        if let Some(err) = resp.error {
            return Err(WalletError::RpcError { code: err.code, message: err.message });
        }

        resp.result.ok_or_else(|| WalletError::RpcParse("missing result field".into()))
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
