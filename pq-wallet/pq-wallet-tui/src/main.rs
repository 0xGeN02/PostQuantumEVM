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

use app::App;
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
        println!("  ↑/↓ or j/k   Navigate transaction list");
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
        // Draw
        terminal.draw(|f| ui::draw(f, app))?;

        // Event polling
        let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // 'r' triggers manual refresh
                    if key.code == crossterm::event::KeyCode::Char('r') {
                        rt.block_on(app.refresh());
                    }
                    events::handle_key(app, key);
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
