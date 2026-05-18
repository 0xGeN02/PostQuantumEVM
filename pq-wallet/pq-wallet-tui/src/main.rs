//! pq-tui — Post-Quantum Ethereum wallet terminal UI.
//!
//! A ratatui-based dashboard showing wallet metadata, balance,
//! transaction history, and network info for the PostQuantumEVM chain.

mod app;
mod events;
mod ui;

use std::io;
use std::time::Instant;

use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use ratatui::Terminal;

use app::{App, PendingExec};
use events::TICK_RATE;

fn main() -> Result<()> {
    // Parse args
    let args: Vec<String> = std::env::args().collect();

    let rpc_url = args
        .iter()
        .position(|a| a == "--rpc")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "http://localhost:8545".to_string());

    let keystore_path = args
        .iter()
        .position(|a| a == "--keystore")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "keystore.json".to_string());

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("pq-tui — Post-Quantum Ethereum wallet TUI");
        println!();
        println!("USAGE:");
        println!("  pq-tui [OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("  --rpc <URL>          JSON-RPC endpoint (default: http://localhost:8545)");
        println!("  --keystore <PATH>    Keystore file path (default: keystore.json)");
        println!("  -h, --help           Show this help");
        println!();
        println!("KEYS:");
        println!("  ←/→ or Tab   Switch between panels");
        println!("  ↑/↓ or j/k   Navigate transaction/block list");
        println!("  s            Send qETH transfer");
        println!("  d            Deploy contract");
        println!("  c            Call contract (read-only)");
        println!("  r            Refresh data");
        println!("  q            Quit");
        return Ok(());
    }

    // Setup terminal
    enable_raw_mode().context("enabling raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and load keystore
    let mut app = App::new(rpc_url, keystore_path);
    app.load_keystore();

    // Run the main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;

    // Initial refresh
    rt.block_on(app.refresh());

    let mut last_tick = Instant::now();

    loop {
        // Draw (needs &mut app for TableState)
        terminal.draw(|f| ui::draw(f, app))?;

        // Event polling
        let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let should_refresh = events::handle_key(app, key);

                    // Execute pending action if any
                    if let Some(exec) = app.pending_exec.take() {
                        match exec {
                            PendingExec::Send { to, value } => {
                                rt.block_on(app.execute_send(&to, &value));
                            }
                            PendingExec::Deploy { code, gas_limit } => {
                                rt.block_on(app.execute_deploy(&code, &gas_limit));
                            }
                            PendingExec::Call { to, data } => {
                                rt.block_on(app.execute_call(&to, &data));
                            }
                        }
                    } else if should_refresh {
                        // Handle search: if search_input is non-empty and
                        // search_mode just ended, execute the search
                        if !app.search_input.is_empty() && !app.search_mode {
                            let query = app.search_input.clone();
                            let needs_refresh = rt.block_on(app.search_block(&query));
                            if needs_refresh {
                                rt.block_on(app.refresh());
                                app.search_input.clear();
                            } else {
                                // Search failed, re-enter search mode to show error
                                app.search_mode = true;
                            }
                        } else if app.showing_address_viewer && !app.address_input.is_empty() && app.address_lookup.is_none() {
                            // Handle address lookup
                            let addr = app.address_input.clone();
                            rt.block_on(app.lookup_address(&addr));
                        } else {
                            rt.block_on(app.refresh());
                        }
                    }
                }
            }
        }

        // Auto-refresh on tick
        if last_tick.elapsed() >= TICK_RATE {
            rt.block_on(app.refresh());
            last_tick = Instant::now();
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
