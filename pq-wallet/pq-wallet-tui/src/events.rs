//! Event handling for the PQ wallet TUI.

use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Tab};

/// Tick interval for auto-refresh (3 seconds).
pub const TICK_RATE: Duration = Duration::from_secs(3);

/// Handle a key event, mutating the app state accordingly.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Quit
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        // Tab navigation
        KeyCode::Right | KeyCode::Tab => app.next_tab(),
        KeyCode::Left | KeyCode::BackTab => app.prev_tab(),

        // Refresh (manual)
        KeyCode::Char('r') => {
            // refresh is triggered in the main loop
        }

        // List navigation (context-dependent)
        KeyCode::Up | KeyCode::Char('k') => match app.active_tab {
            Tab::Transactions => {
                if app.tx_selected > 0 {
                    app.tx_selected -= 1;
                }
            }
            Tab::Blocks => {
                if app.block_selected > 0 {
                    app.block_selected -= 1;
                }
            }
            _ => {}
        },
        KeyCode::Down | KeyCode::Char('j') => match app.active_tab {
            Tab::Transactions => {
                if app.tx_selected + 1 < app.transactions.len() {
                    app.tx_selected += 1;
                }
            }
            Tab::Blocks => {
                if app.block_selected + 1 < app.blocks.len() {
                    app.block_selected += 1;
                }
            }
            _ => {}
        },

        _ => {}
    }
}
