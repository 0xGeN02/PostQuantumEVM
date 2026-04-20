//! pq-wallet — Post-quantum Ethereum wallet CLI
//!
//! Commands:
//!   new      Generate a new ML-DSA-65 keypair and save to keystore
//!   address  Show the address stored in a keystore (no passphrase needed)
//!   balance  Query ETH balance via JSON-RPC
//!   send     Build, sign and broadcast a PQ transaction
//!   sign     Sign an arbitrary message and print the hex signature

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use pq_wallet_core::{Keystore, PqKeypair, PqSigner, RpcClient, tx::PqTxRequest};

// ─── CLI definition ───────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "pq-wallet",
    about = "Post-quantum Ethereum wallet (ML-DSA-65 / CRYSTALS-Dilithium)",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a new ML-DSA-65 keypair and save to an encrypted keystore file.
    New {
        /// Output keystore file path.
        #[arg(short, long, default_value = "keystore.json")]
        output: PathBuf,

        /// Passphrase to encrypt the keystore.
        /// If not provided, you will be prompted.
        #[arg(short, long)]
        passphrase: Option<String>,
    },

    /// Show the Ethereum address in a keystore (no passphrase required).
    Address {
        /// Keystore file path.
        #[arg(short, long, default_value = "keystore.json")]
        keystore: PathBuf,
    },

    /// Query the ETH balance of the wallet address.
    Balance {
        /// Keystore file path.
        #[arg(short, long, default_value = "keystore.json")]
        keystore: PathBuf,

        /// JSON-RPC endpoint URL.
        #[arg(short, long, default_value = "http://localhost:8545")]
        rpc: String,
    },

    /// Build, sign and broadcast a post-quantum transaction.
    Send {
        /// Keystore file path.
        #[arg(short, long, default_value = "keystore.json")]
        keystore: PathBuf,

        /// Passphrase to decrypt the keystore.
        #[arg(short, long)]
        passphrase: Option<String>,

        /// Recipient address (hex, with or without 0x prefix).
        #[arg(long)]
        to: String,

        /// Value to send in wei.
        #[arg(long, default_value = "0")]
        value: u128,

        /// Gas limit (default: 21000).
        #[arg(long, default_value = "21000")]
        gas_limit: u64,

        /// Gas price in wei. If not set, fetched from node.
        #[arg(long)]
        gas_price: Option<u128>,

        /// Calldata (hex, optional).
        #[arg(long, default_value = "")]
        data: String,

        /// JSON-RPC endpoint URL.
        #[arg(short, long, default_value = "http://localhost:8545")]
        rpc: String,

        /// Dry run — print the signed tx hex without broadcasting.
        #[arg(long)]
        dry_run: bool,
    },

    /// Sign an arbitrary message and print the hex signature.
    Sign {
        /// Keystore file path.
        #[arg(short, long, default_value = "keystore.json")]
        keystore: PathBuf,

        /// Passphrase to decrypt the keystore.
        #[arg(short, long)]
        passphrase: Option<String>,

        /// Message to sign (UTF-8 string).
        message: String,
    },
}

// ─── Entry point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::New { output, passphrase } => cmd_new(output, passphrase).await,
        Command::Address { keystore } => cmd_address(keystore),
        Command::Balance { keystore, rpc } => cmd_balance(keystore, rpc).await,
        Command::Send { keystore, passphrase, to, value, gas_limit, gas_price, data, rpc, dry_run } => {
            cmd_send(keystore, passphrase, to, value, gas_limit, gas_price, data, rpc, dry_run).await
        }
        Command::Sign { keystore, passphrase, message } => cmd_sign(keystore, passphrase, message),
    }
}

// ─── Command handlers ─────────────────────────────────────────────────────────

async fn cmd_new(output: PathBuf, passphrase: Option<String>) -> Result<()> {
    let pass = resolve_passphrase(passphrase, true)?;

    print!("Generating ML-DSA-65 keypair... ");
    let keypair = PqKeypair::generate();
    let address = keypair.address();
    println!("done.");

    keypair.save(&output, &pass).with_context(|| format!("saving keystore to {}", output.display()))?;

    println!("Keystore saved to:  {}", output.display());
    println!("Address:            {address}");
    println!();
    println!("WARNING: Anyone with your passphrase and keystore file can steal your funds.");
    println!("         Keep them separate and make a backup.");

    Ok(())
}

fn cmd_address(keystore: PathBuf) -> Result<()> {
    let address = Keystore::address_from_file(&keystore)
        .with_context(|| format!("reading keystore {}", keystore.display()))?;
    println!("{address}");
    Ok(())
}

async fn cmd_balance(keystore: PathBuf, rpc: String) -> Result<()> {
    let address_str = Keystore::address_from_file(&keystore)
        .with_context(|| format!("reading keystore {}", keystore.display()))?;

    let address = parse_address(&address_str)?;
    let client = RpcClient::new(&rpc);

    let balance_wei = client.get_balance(address).await
        .with_context(|| format!("querying balance from {rpc}"))?;

    let balance_eth = balance_wei as f64 / 1e18;

    println!("Address: {address}");
    println!("Balance: {balance_eth:.6} ETH ({balance_wei} wei)");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn cmd_send(
    keystore: PathBuf,
    passphrase: Option<String>,
    to: String,
    value: u128,
    gas_limit: u64,
    gas_price: Option<u128>,
    data: String,
    rpc: String,
    dry_run: bool,
) -> Result<()> {
    let pass = resolve_passphrase(passphrase, false)?;
    let keypair = Keystore::load(&keystore, &pass)
        .with_context(|| format!("decrypting keystore {}", keystore.display()))?;

    let client = RpcClient::new(&rpc);

    // Fetch chain_id, nonce, gas_price from node (unless dry_run skips RPC)
    let (chain_id, nonce, gas_price) = if dry_run && gas_price.is_some() {
        (1u64, 0u64, gas_price.unwrap())
    } else {
        let chain_id = client.chain_id().await.context("fetching chain_id")?;
        let nonce = client.get_nonce(keypair.address()).await.context("fetching nonce")?;
        let gp = match gas_price {
            Some(p) => p,
            None => client.gas_price().await.context("fetching gas_price")?,
        };
        (chain_id, nonce, gp)
    };

    let to_addr = parse_address(&to)?;
    let input = if data.is_empty() {
        vec![]
    } else {
        hex::decode(data.strip_prefix("0x").unwrap_or(&data)).context("decoding calldata hex")?
    };

    let tx = PqTxRequest {
        chain_id,
        nonce,
        to: Some(to_addr),
        value,
        gas_limit,
        gas_price,
        input,
    };

    let signer = PqSigner::new(&keypair);
    let signed = signer.sign(tx);

    let raw_hex = format!("0x{}", hex::encode(signed.encode()));

    if dry_run {
        println!("Tx hash (local):  {}", signed.hash);
        println!("Chain ID:         {chain_id}");
        println!("Nonce:            {nonce}");
        println!("Gas price:        {gas_price} wei");
        println!("Gas limit:        {gas_limit}");
        println!("To:               {to_addr}");
        println!("Value:            {value} wei");
        println!("Raw tx size:      {} bytes", signed.encode().len());
        println!("Raw tx hex:       {}...", &raw_hex[..std::cmp::min(80, raw_hex.len())]);
        return Ok(());
    }

    print!("Broadcasting transaction... ");
    let tx_hash = client.send_raw_transaction(&raw_hex).await.context("broadcasting transaction")?;
    println!("done.");
    println!("Transaction hash: {tx_hash}");

    Ok(())
}

fn cmd_sign(keystore: PathBuf, passphrase: Option<String>, message: String) -> Result<()> {
    let pass = resolve_passphrase(passphrase, false)?;
    let keypair = Keystore::load(&keystore, &pass)
        .with_context(|| format!("decrypting keystore {}", keystore.display()))?;

    let sig_bytes = keypair.sign_message(message.as_bytes());
    let sig_hex = hex::encode(&sig_bytes);

    println!("Message:   {message}");
    println!("Signer:    {}", keypair.address());
    println!("Signature: {sig_hex}");
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Prompt for a passphrase if not provided via flag.
fn resolve_passphrase(provided: Option<String>, confirm: bool) -> Result<String> {
    if let Some(p) = provided {
        return Ok(p);
    }

    let pass = rpassword_prompt("Enter passphrase: ")?;
    if confirm {
        let pass2 = rpassword_prompt("Confirm passphrase: ")?;
        if pass != pass2 {
            bail!("Passphrases do not match.");
        }
    }
    Ok(pass)
}

/// Read a passphrase from stdin without echoing (simple version using print+stdin).
fn rpassword_prompt(prompt: &str) -> Result<String> {
    use std::io::{self, Write};
    print!("{prompt}");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

/// Parse a hex address string (with or without 0x prefix) into `Address`.
fn parse_address(s: &str) -> Result<alloy_primitives::Address> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).context("invalid address hex")?;
    if bytes.len() != 20 {
        bail!("address must be 20 bytes, got {}", bytes.len());
    }
    Ok(alloy_primitives::Address::from_slice(&bytes))
}
