//! Event handling for the PQ wallet TUI.

use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{ActionMode, App, PendingActionKind, PendingExec, Tab};

/// Tick interval for auto-refresh (3 seconds).
pub const TICK_RATE: Duration = Duration::from_secs(3);

/// Handle a key event, mutating the app state accordingly.
/// Returns `true` if a manual refresh should be triggered.
pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    // If we're in search mode, handle search input
    if app.search_mode {
        return handle_search_input(app, key);
    }

    // If we're in address viewer mode
    if app.showing_address_viewer {
        return handle_address_viewer_input(app, key);
    }

    // If we're in action input mode, handle text input
    if app.asking_passphrase {
        return handle_passphrase_input(app, key);
    }

    if app.action != ActionMode::None {
        return handle_action_input(app, key);
    }

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
        KeyCode::Char('r') => return true,

        // ─── Search (/) ───
        KeyCode::Char('/') => {
            app.search_mode = true;
            app.search_input.clear();
            app.search_error = None;
        }

        // ─── Address viewer (a) ───
        KeyCode::Char('a') => {
            app.showing_address_viewer = true;
            app.address_input.clear();
            app.address_lookup = None;
        }

        // ─── Action hotkeys ───
        // 's' = Send transfer
        KeyCode::Char('s') => {
            if app.passphrase.is_none() {
                app.asking_passphrase = true;
                app.passphrase_input.clear();
                app.pending_action_kind = Some(PendingActionKind::Send);
            } else {
                app.action = ActionMode::Send {
                    field: 0,
                    to: String::new(),
                    value: String::new(),
                };
            }
        }
        // 'd' = Deploy contract
        KeyCode::Char('d') => {
            if app.passphrase.is_none() {
                app.asking_passphrase = true;
                app.passphrase_input.clear();
                app.pending_action_kind = Some(PendingActionKind::Deploy);
            } else {
                app.action = ActionMode::Deploy {
                    field: 0,
                    code: String::new(),
                    gas_limit: "1000000".to_string(),
                };
            }
        }
        // 'c' = Call contract (read-only, no passphrase needed)
        KeyCode::Char('c') => {
            app.action = ActionMode::Call {
                field: 0,
                to: String::new(),
                data: String::new(),
            };
        }

        // List navigation (context-dependent)
        KeyCode::Up | KeyCode::Char('k') => match app.active_tab {
            Tab::Transactions if app.tx_selected > 0 => {
                app.tx_selected -= 1;
            }
            Tab::Blocks if app.block_selected > 0 => {
                app.block_selected -= 1;
            }
            _ => {}
        },
        KeyCode::Down | KeyCode::Char('j') => match app.active_tab {
            Tab::Transactions if app.tx_selected + 1 < app.transactions.len() => {
                app.tx_selected += 1;
            }
            Tab::Blocks if app.block_selected + 1 < app.blocks.len() => {
                app.block_selected += 1;
            }
            _ => {}
        },

        // ─── Block pagination ───
        // Home: jump to latest
        KeyCode::Home if app.active_tab == Tab::Blocks => {
            app.block_page_end = None;
            app.block_selected = 0;
            return true;
        }
        // PgUp / '[': older blocks
        KeyCode::PageUp | KeyCode::Char('[') if app.active_tab == Tab::Blocks => {
            app.blocks_page_prev();
            return true;
        }
        // PgDn / ']': newer blocks
        KeyCode::PageDown | KeyCode::Char(']') if app.active_tab == Tab::Blocks => {
            app.blocks_page_next();
            return true;
        }

        _ => {}
    }
    false
}

fn handle_search_input(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.search_mode = false;
            app.search_input.clear();
            app.search_error = None;
        }
        KeyCode::Enter => {
            // We can't call async here, so store the query for the main loop
            // to handle. We use pending_exec-like pattern.
            // For simplicity, mark search_mode = false and return true to trigger refresh
            // The main loop will call search_block.
            app.search_mode = false;
            return true; // signal that a search-triggered refresh is needed
        }
        KeyCode::Backspace => {
            app.search_input.pop();
            app.search_error = None;
        }
        KeyCode::Char(c) => {
            app.search_input.push(c);
            app.search_error = None;
        }
        _ => {}
    }
    false
}

fn handle_address_viewer_input(app: &mut App, key: KeyEvent) -> bool {
    // If we have a result, any key dismisses it
    if app.address_lookup.is_some() {
        app.showing_address_viewer = false;
        app.address_lookup = None;
        app.address_input.clear();
        return false;
    }

    match key.code {
        KeyCode::Esc => {
            app.showing_address_viewer = false;
            app.address_input.clear();
        }
        KeyCode::Enter => {
            // Signal main loop to perform the address lookup
            app.showing_address_viewer = true;
            return true; // trigger async address lookup
        }
        KeyCode::Backspace => {
            app.address_input.pop();
        }
        KeyCode::Char(c) => {
            app.address_input.push(c);
        }
        _ => {}
    }
    false
}

fn handle_passphrase_input(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Enter => {
            app.passphrase = Some(app.passphrase_input.clone());
            app.passphrase_input.clear();
            app.asking_passphrase = false;
            // Start the action that triggered the passphrase prompt
            match app.pending_action_kind.take() {
                Some(PendingActionKind::Send) => {
                    app.action = ActionMode::Send {
                        field: 0,
                        to: String::new(),
                        value: String::new(),
                    };
                }
                Some(PendingActionKind::Deploy) => {
                    app.action = ActionMode::Deploy {
                        field: 0,
                        code: String::new(),
                        gas_limit: "1000000".to_string(),
                    };
                }
                None => {
                    // Shouldn't happen, but default to Send
                    app.action = ActionMode::Send {
                        field: 0,
                        to: String::new(),
                        value: String::new(),
                    };
                }
            }
        }
        KeyCode::Esc => {
            app.asking_passphrase = false;
            app.passphrase_input.clear();
            app.pending_action_kind = None;
        }
        KeyCode::Backspace => {
            app.passphrase_input.pop();
        }
        KeyCode::Char(c) => {
            app.passphrase_input.push(c);
        }
        _ => {}
    }
    false
}

fn handle_action_input(app: &mut App, key: KeyEvent) -> bool {
    // Escape cancels any action
    if key.code == KeyCode::Esc {
        app.action = ActionMode::None;
        return false;
    }

    // If showing a result, any key dismisses it
    if matches!(app.action, ActionMode::Result { .. }) {
        app.action = ActionMode::None;
        return true; // refresh after action
    }

    match &mut app.action {
        ActionMode::Send { field, to, value } => match key.code {
            KeyCode::Enter => {
                if *field == 0 {
                    *field = 1;
                } else {
                    // Submit: store pending execution
                    let exec = PendingExec::Send {
                        to: to.clone(),
                        value: value.clone(),
                    };
                    app.pending_exec = Some(exec);
                    app.action = ActionMode::None;
                    return true;
                }
            }
            KeyCode::Backspace => {
                if *field == 0 {
                    to.pop();
                } else {
                    value.pop();
                }
            }
            KeyCode::Char(c) => {
                if *field == 0 {
                    to.push(c);
                } else {
                    value.push(c);
                }
            }
            _ => {}
        },
        ActionMode::Deploy { field, code, gas_limit } => match key.code {
            KeyCode::Enter => {
                if *field == 0 {
                    *field = 1;
                } else {
                    let exec = PendingExec::Deploy {
                        code: code.clone(),
                        gas_limit: gas_limit.clone(),
                    };
                    app.pending_exec = Some(exec);
                    app.action = ActionMode::None;
                    return true;
                }
            }
            KeyCode::Backspace => {
                if *field == 0 {
                    code.pop();
                } else {
                    gas_limit.pop();
                }
            }
            KeyCode::Char(c) => {
                if *field == 0 {
                    code.push(c);
                } else {
                    gas_limit.push(c);
                }
            }
            _ => {}
        },
        ActionMode::Call { field, to, data } => match key.code {
            KeyCode::Enter => {
                if *field == 0 {
                    *field = 1;
                } else {
                    let exec = PendingExec::Call {
                        to: to.clone(),
                        data: data.clone(),
                    };
                    app.pending_exec = Some(exec);
                    app.action = ActionMode::None;
                    return true;
                }
            }
            KeyCode::Backspace => {
                if *field == 0 {
                    to.pop();
                } else {
                    data.pop();
                }
            }
            KeyCode::Char(c) => {
                if *field == 0 {
                    to.push(c);
                } else {
                    data.push(c);
                }
            }
            _ => {}
        },
        _ => {}
    }
    false
}
