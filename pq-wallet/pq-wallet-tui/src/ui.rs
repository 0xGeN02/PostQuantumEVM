//! UI rendering for the PQ wallet TUI.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Tabs},
    Frame,
};

use crate::app::{ActionMode, App, Tab};

/// Render the full UI.
pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tabs
            Constraint::Min(0),   // content
            Constraint::Length(3), // status bar
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);

    match app.active_tab {
        Tab::Wallet => draw_wallet_tab(f, app, chunks[1]),
        Tab::Transactions => draw_transactions_tab(f, app, chunks[1]),
        Tab::Blocks => draw_blocks_tab(f, app, chunks[1]),
        Tab::Network => draw_network_tab(f, app, chunks[1]),
    }

    draw_status_bar(f, app, chunks[2]);

    // ─── Overlays (drawn on top) ───
    if app.asking_passphrase {
        draw_passphrase_overlay(f, app);
    } else if app.action != ActionMode::None {
        draw_action_overlay(f, app);
    }
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|t| {
            let style = if *t == app.active_tab {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(Span::styled(t.title(), style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" PQ Wallet — Post-Quantum Ethereum "),
        )
        .highlight_style(Style::default().fg(Color::Cyan))
        .select(app.active_tab as usize);

    f.render_widget(tabs, area);
}

fn draw_wallet_tab(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // wallet info
            Constraint::Length(7),  // balance
            Constraint::Min(0),    // key comparison
        ])
        .split(area);

    // ─── Wallet Info ───
    let address_str = app
        .address
        .map(|a| format!("{a:?}"))
        .unwrap_or_else(|| "Not loaded".to_string());

    let info_text = vec![
        Line::from(vec![
            Span::styled("  Address:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(&address_str, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Algorithm:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(app.algorithm, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("  Keystore:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(&app.keystore_path, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  PK size:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} bytes", app.pk_size), Style::default().fg(Color::Magenta)),
        ]),
        Line::from(vec![
            Span::styled("  Sig size:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} bytes", app.sig_size), Style::default().fg(Color::Magenta)),
        ]),
        Line::from(vec![
            Span::styled("  Addr hash:  ", Style::default().fg(Color::DarkGray)),
            Span::styled("SHAKE-256(pk)[12..]", Style::default().fg(Color::Cyan)),
        ]),
    ];

    let info = Paragraph::new(info_text)
        .block(Block::default().borders(Borders::ALL).title(" Wallet Identity "));
    f.render_widget(info, chunks[0]);

    // ─── Balance ───
    let bal_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Balance:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                app.balance_qeth(),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("            ", Style::default()),
            Span::styled(app.balance_wei_str(), Style::default().fg(Color::DarkGray)),
        ]),
    ];

    let balance = Paragraph::new(bal_text)
        .block(Block::default().borders(Borders::ALL).title(" Balance "));
    f.render_widget(balance, chunks[1]);

    // ─── Key Size Comparison + Hash Display ───
    let keccak_hash = app.keccak256_hash.as_deref().unwrap_or("N/A");
    let shake_hash = app.shake256_hash.as_deref().unwrap_or("N/A");
    let addr_keccak = app.addr_keccak256.as_deref().unwrap_or("N/A");
    let addr_shake = app.addr_shake256.as_deref().unwrap_or("N/A");

    let comparison = vec![
        Line::from(Span::styled(
            "  Classical (ECDSA/secp256k1) vs Post-Quantum (ML-DSA-65)",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Property         ", Style::default().fg(Color::DarkGray)),
            Span::styled("ECDSA          ", Style::default().fg(Color::Red)),
            Span::styled("ML-DSA-65", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("  ─────────────────", Style::default().fg(Color::DarkGray)),
            Span::styled("───────────────", Style::default().fg(Color::DarkGray)),
            Span::styled("─────────────────", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  Public Key       ", Style::default().fg(Color::DarkGray)),
            Span::styled("64 bytes       ", Style::default().fg(Color::Red)),
            Span::styled("1,952 bytes (30x)", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("  Signature        ", Style::default().fg(Color::DarkGray)),
            Span::styled("65 bytes       ", Style::default().fg(Color::Red)),
            Span::styled("3,309 bytes (51x)", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("  Tx size (typ.)   ", Style::default().fg(Color::DarkGray)),
            Span::styled("~110 bytes     ", Style::default().fg(Color::Red)),
            Span::styled("~5,400 bytes", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("  Address derivat. ", Style::default().fg(Color::DarkGray)),
            Span::styled("keccak256      ", Style::default().fg(Color::Red)),
            Span::styled("SHAKE-256", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("  Quantum-safe     ", Style::default().fg(Color::DarkGray)),
            Span::styled("NO             ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled("YES", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ─── Hash of YOUR public key (1952 bytes) ───",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  keccak256(pk): ", Style::default().fg(Color::Red)),
            Span::styled(keccak_hash, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  shake256(pk):  ", Style::default().fg(Color::Green)),
            Span::styled(shake_hash, Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ─── Derived addresses (last 20 bytes of hash) ───",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Classical:  ", Style::default().fg(Color::Red)),
            Span::styled(addr_keccak, Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled("  ← would be if Ethereum", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  PQ (ours):  ", Style::default().fg(Color::Green)),
            Span::styled(addr_shake, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled("  ← actual address", Style::default().fg(Color::DarkGray)),
        ]),
    ];

    let cmp = Paragraph::new(comparison)
        .block(Block::default().borders(Borders::ALL).title(" PQ vs Classical Comparison "));
    f.render_widget(cmp, chunks[2]);
}

fn draw_transactions_tab(f: &mut Frame, app: &App, area: Rect) {
    if app.transactions.is_empty() {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No transactions found.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Send a transaction with pq-wallet send, then refresh.",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(Block::default().borders(Borders::ALL).title(" Transactions "));
        f.render_widget(msg, area);
        return;
    }

    // Split: table on top, detail on bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),     // tx list
            Constraint::Length(12), // tx detail
        ])
        .split(area);

    let header_cells = ["Hash", "Block", "Kind", "To/Contract", "Value", "Type"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let rows = app.transactions.iter().enumerate().map(|(i, tx)| {
        let style = if i == app.tx_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let hash_short = if tx.hash.len() > 14 {
            format!("{}..{}", &tx.hash[..8], &tx.hash[tx.hash.len() - 4..])
        } else {
            tx.hash.clone()
        };

        let kind = match tx.kind() {
            crate::app::TxKind::Transfer => "TRANSFER",
            crate::app::TxKind::Deploy => "DEPLOY",
            crate::app::TxKind::ContractCall => "CALL",
        };

        let to_display = match tx.kind() {
            crate::app::TxKind::Deploy => {
                tx.contract_address.as_ref().map(|a| {
                    if a.len() > 14 { format!("→{}..{}", &a[..8], &a[a.len()-4..]) }
                    else { format!("→{a}") }
                }).unwrap_or_else(|| "CREATING...".to_string())
            }
            _ => {
                tx.to.as_ref().map(|t| {
                    if t.len() > 14 { format!("{}..{}", &t[..8], &t[t.len()-4..]) }
                    else { t.clone() }
                }).unwrap_or_else(|| "—".to_string())
            }
        };

        let value_display = format_value_qeth(&tx.value_wei);

        Row::new(vec![
            Cell::from(hash_short),
            Cell::from(tx.block.clone()),
            Cell::from(kind),
            Cell::from(to_display),
            Cell::from(value_display),
            Cell::from(tx.tx_type.clone()),
        ])
        .style(style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(16),
            Constraint::Min(12),
            Constraint::Length(6),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" Transactions (PQ type 0x50) "));

    f.render_widget(table, chunks[0]);

    // ─── Tx detail panel ───
    if let Some(tx) = app.transactions.get(app.tx_selected) {
        let kind_str = match tx.kind() {
            crate::app::TxKind::Transfer => "Value Transfer",
            crate::app::TxKind::Deploy => "Contract Deployment",
            crate::app::TxKind::ContractCall => "Contract Call",
        };
        let kind_color = match tx.kind() {
            crate::app::TxKind::Transfer => Color::Green,
            crate::app::TxKind::Deploy => Color::Magenta,
            crate::app::TxKind::ContractCall => Color::Yellow,
        };

        let mut detail = vec![
            Line::from(vec![
                Span::styled("  Kind:       ", Style::default().fg(Color::DarkGray)),
                Span::styled(kind_str, Style::default().fg(kind_color).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Hash:       ", Style::default().fg(Color::DarkGray)),
                Span::styled(&tx.hash, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("  From:       ", Style::default().fg(Color::DarkGray)),
                Span::styled(&tx.from, Style::default().fg(Color::White)),
            ]),
        ];

        // To / Contract address
        match tx.kind() {
            crate::app::TxKind::Deploy => {
                if let Some(ref addr) = tx.contract_address {
                    detail.push(Line::from(vec![
                        Span::styled("  Contract:   ", Style::default().fg(Color::DarkGray)),
                        Span::styled(addr, Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                    ]));
                }
                detail.push(Line::from(vec![
                    Span::styled("  Init code:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{} bytes", tx.calldata_size()), Style::default().fg(Color::White)),
                ]));
            }
            crate::app::TxKind::ContractCall => {
                detail.push(Line::from(vec![
                    Span::styled("  To:         ", Style::default().fg(Color::DarkGray)),
                    Span::styled(tx.to.as_deref().unwrap_or("—"), Style::default().fg(Color::White)),
                ]));
                if let Some(selector) = tx.function_selector() {
                    detail.push(Line::from(vec![
                        Span::styled("  Selector:   ", Style::default().fg(Color::DarkGray)),
                        Span::styled(selector, Style::default().fg(Color::Yellow)),
                        Span::styled(format!("  ({} bytes calldata)", tx.calldata_size()), Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
            crate::app::TxKind::Transfer => {
                detail.push(Line::from(vec![
                    Span::styled("  To:         ", Style::default().fg(Color::DarkGray)),
                    Span::styled(tx.to.as_deref().unwrap_or("—"), Style::default().fg(Color::White)),
                ]));
            }
        }

        detail.push(Line::from(vec![
            Span::styled("  Value:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(format_value_qeth(&tx.value_wei), Style::default().fg(Color::Green)),
            Span::styled(format!("  ({})", tx.value_wei), Style::default().fg(Color::DarkGray)),
        ]));
        detail.push(Line::from(vec![
            Span::styled("  Sig size:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} bytes (ML-DSA-65)", tx.sig_size), Style::default().fg(Color::Magenta)),
        ]));

        let detail_widget = Paragraph::new(detail)
            .block(Block::default().borders(Borders::ALL).title(" Tx Detail (↑/↓ to navigate) "));
        f.render_widget(detail_widget, chunks[1]);
    }
}

fn draw_blocks_tab(f: &mut Frame, app: &App, area: Rect) {
    if app.blocks.is_empty() {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No blocks found. Is the node running?",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(Block::default().borders(Borders::ALL).title(" Blocks "));
        f.render_widget(msg, area);
        return;
    }

    // Split: table on top, detail on bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),    // block list
            Constraint::Length(10), // block detail
        ])
        .split(area);

    // ─── Block table ───
    let header_cells = ["#", "Hash", "Txs", "Gas Used", "Gas %", "Base Fee", "Timestamp"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let rows = app.blocks.iter().enumerate().map(|(i, blk)| {
        let style = if i == app.block_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let hash_short = if blk.hash.len() > 14 {
            format!("{}..{}", &blk.hash[..8], &blk.hash[blk.hash.len() - 4..])
        } else {
            blk.hash.clone()
        };

        let gas_pct = if blk.gas_limit > 0 {
            format!("{:.1}%", (blk.gas_used as f64 / blk.gas_limit as f64) * 100.0)
        } else {
            "0%".to_string()
        };

        let base_fee_gwei = blk.base_fee as f64 / 1e9;
        let timestamp_str = format_timestamp(blk.timestamp);

        Row::new(vec![
            Cell::from(format!("{}", blk.number)),
            Cell::from(hash_short),
            Cell::from(format!("{}", blk.tx_count)),
            Cell::from(format_gas(blk.gas_used)),
            Cell::from(gas_pct),
            Cell::from(format!("{:.2} Gwei", base_fee_gwei)),
            Cell::from(timestamp_str),
        ])
        .style(style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(7),
            Constraint::Length(14),
            Constraint::Length(5),
            Constraint::Length(10),
            Constraint::Length(7),
            Constraint::Length(12),
            Constraint::Min(12),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(format!(
        " Blocks — Latest #{} ",
        app.block_number
    )));

    f.render_widget(table, chunks[0]);

    // ─── Block detail panel ───
    if let Some(blk) = app.blocks.get(app.block_selected) {
        let detail = vec![
            Line::from(vec![
                Span::styled("  Block:     ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("#{}", blk.number), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Hash:      ", Style::default().fg(Color::DarkGray)),
                Span::styled(&blk.hash, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("  Miner:     ", Style::default().fg(Color::DarkGray)),
                Span::styled(&blk.miner, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("  Gas:       ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{} / {} ({:.1}%)", format_gas(blk.gas_used), format_gas(blk.gas_limit),
                        (blk.gas_used as f64 / blk.gas_limit.max(1) as f64) * 100.0),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Base fee:  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:.4} Gwei ({} wei)", blk.base_fee as f64 / 1e9, blk.base_fee),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Txs:       ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}", blk.tx_count), Style::default().fg(Color::Green)),
            ]),
        ];

        let detail_widget = Paragraph::new(detail)
            .block(Block::default().borders(Borders::ALL).title(" Block Detail (↑/↓ to navigate) "));
        f.render_widget(detail_widget, chunks[1]);
    }
}

fn draw_network_tab(f: &mut Frame, app: &App, area: Rect) {
    let status_color = if app.connected { Color::Green } else { Color::Red };
    let status_text = if app.connected { "CONNECTED" } else { "DISCONNECTED" };

    let info = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Status:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(status_text, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  RPC endpoint: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&app.rpc_url, Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Chain ID:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} (0x{:x})", app.chain_id, app.chain_id),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Chain name:   ", Style::default().fg(Color::DarkGray)),
            Span::styled("PostQuantumEVM", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("  Native token: ", Style::default().fg(Color::DarkGray)),
            Span::styled("qETH (Quantum Ethereum)", Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Block number: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", app.block_number), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Gas price:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} wei ({:.2} Gwei)", app.gas_price, app.gas_price as f64 / 1e9),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ─── Consensus ───",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::styled("  Algorithm:    ", Style::default().fg(Color::DarkGray)),
            Span::styled("PoA (Proof of Authority)", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("  Seal type:    ", Style::default().fg(Color::DarkGray)),
            Span::styled("ML-DSA-65 block signature", Style::default().fg(Color::Magenta)),
        ]),
        Line::from(vec![
            Span::styled("  Rotation:     ", Style::default().fg(Color::DarkGray)),
            Span::styled("Round-robin validator set", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ─── Fee Model ───",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::styled("  Model:        ", Style::default().fg(Color::DarkGray)),
            Span::styled("EIP-1559 (base fee burned, priority to validator)", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Block reward: ", Style::default().fg(Color::DarkGray)),
            Span::styled("None (PoA — no inflation)", Style::default().fg(Color::White)),
        ]),
    ];

    let network = Paragraph::new(info)
        .block(Block::default().borders(Borders::ALL).title(" Network — PostQuantumEVM "));
    f.render_widget(network, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status = Line::from(vec![
        Span::styled(" ←/→ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Tab  ", Style::default().fg(Color::DarkGray)),
        Span::styled(" s ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::styled("Send  ", Style::default().fg(Color::DarkGray)),
        Span::styled(" d ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        Span::styled("Deploy  ", Style::default().fg(Color::DarkGray)),
        Span::styled(" c ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled("Call  ", Style::default().fg(Color::DarkGray)),
        Span::styled(" r ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Refresh  ", Style::default().fg(Color::DarkGray)),
        Span::styled(" q ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Quit  ", Style::default().fg(Color::DarkGray)),
        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            if app.connected { "●" } else { "○" },
            Style::default().fg(if app.connected { Color::Green } else { Color::Red }),
        ),
        Span::styled(
            if app.connected { " Connected" } else { " Disconnected" },
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let bar = Paragraph::new(status)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(bar, area);
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Format gas in human-readable form (e.g. "21.00K", "1.20M").
fn format_gas(gas: u64) -> String {
    if gas >= 1_000_000 {
        format!("{:.2}M", gas as f64 / 1_000_000.0)
    } else if gas >= 1_000 {
        format!("{:.1}K", gas as f64 / 1_000.0)
    } else {
        format!("{gas}")
    }
}

/// Format a UNIX timestamp as a relative or absolute time string.
fn format_timestamp(ts: u64) -> String {
    if ts == 0 {
        return "genesis".to_string();
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if now > ts {
        let diff = now - ts;
        if diff < 60 {
            format!("{diff}s ago")
        } else if diff < 3600 {
            format!("{}m ago", diff / 60)
        } else {
            format!("{}h ago", diff / 3600)
        }
    } else {
        format!("t={ts}")
    }
}

/// Convert a hex wei value (e.g. "0xde0b6b3a7640000") to a human-readable string.
/// Shows qETH if >= 0.001, otherwise shows wei.
fn format_value_qeth(hex_val: &str) -> String {
    let s = hex_val.strip_prefix("0x").unwrap_or(hex_val);
    let wei = u128::from_str_radix(s, 16).unwrap_or(0);

    if wei == 0 {
        return "0 qETH".to_string();
    }

    let qeth = wei as f64 / 1e18;
    if qeth >= 0.001 {
        format!("{qeth:.6} qETH")
    } else {
        // Show in Gwei for small amounts
        let gwei = wei as f64 / 1e9;
        format!("{gwei:.2} Gwei")
    }
}

// ─── Overlays ────────────────────────────────────────────────────────────────

/// Calculate a centered rect of given width/height within `area`.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

/// Draw the passphrase input overlay.
fn draw_passphrase_overlay(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 7, f.area());
    f.render_widget(Clear, area);

    let masked: String = "*".repeat(app.passphrase_input.len());
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Enter keystore passphrase:",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  > ", Style::default().fg(Color::Cyan)),
            Span::styled(masked, Style::default().fg(Color::Green)),
            Span::styled("█", Style::default().fg(Color::White)),
        ]),
        Line::from(Span::styled(
            "  [Enter] Confirm  [Esc] Cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let popup = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Passphrase "),
    );
    f.render_widget(popup, area);
}

/// Draw the action input overlay (Send / Deploy / Call / Result).
fn draw_action_overlay(f: &mut Frame, app: &App) {
    match &app.action {
        ActionMode::Send { field, to, value } => {
            let area = centered_rect(70, 10, f.area());
            f.render_widget(Clear, area);

            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Send qETH Transfer",
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  To:    ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        to.as_str(),
                        if *field == 0 {
                            Style::default().fg(Color::White)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    ),
                    if *field == 0 {
                        Span::styled("█", Style::default().fg(Color::White))
                    } else {
                        Span::raw("")
                    },
                ]),
                Line::from(vec![
                    Span::styled("  Value: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        value.as_str(),
                        if *field == 1 {
                            Style::default().fg(Color::White)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    ),
                    if *field == 1 {
                        Span::styled("█", Style::default().fg(Color::White))
                    } else {
                        Span::raw("")
                    },
                    Span::styled(" wei", Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "  [Enter] Next/Submit  [Esc] Cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ];

            let popup = Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green))
                    .title(" Send Transfer "),
            );
            f.render_widget(popup, area);
        }
        ActionMode::Deploy { field, code, gas_limit } => {
            let area = centered_rect(70, 10, f.area());
            f.render_widget(Clear, area);

            let code_display = if code.len() > 40 {
                format!("{}...({} bytes)", &code[..40], code.len() / 2)
            } else {
                code.clone()
            };

            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Deploy Contract",
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Code:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        code_display,
                        if *field == 0 {
                            Style::default().fg(Color::White)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    ),
                    if *field == 0 {
                        Span::styled("█", Style::default().fg(Color::White))
                    } else {
                        Span::raw("")
                    },
                ]),
                Line::from(vec![
                    Span::styled("  Gas:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        gas_limit.as_str(),
                        if *field == 1 {
                            Style::default().fg(Color::White)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    ),
                    if *field == 1 {
                        Span::styled("█", Style::default().fg(Color::White))
                    } else {
                        Span::raw("")
                    },
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "  [Enter] Next/Submit  [Esc] Cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ];

            let popup = Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Magenta))
                    .title(" Deploy Contract "),
            );
            f.render_widget(popup, area);
        }
        ActionMode::Call { field, to, data } => {
            let area = centered_rect(70, 10, f.area());
            f.render_widget(Clear, area);

            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Contract Call (read-only)",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  To:    ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        to.as_str(),
                        if *field == 0 {
                            Style::default().fg(Color::White)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    ),
                    if *field == 0 {
                        Span::styled("█", Style::default().fg(Color::White))
                    } else {
                        Span::raw("")
                    },
                ]),
                Line::from(vec![
                    Span::styled("  Data:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        data.as_str(),
                        if *field == 1 {
                            Style::default().fg(Color::White)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    ),
                    if *field == 1 {
                        Span::styled("█", Style::default().fg(Color::White))
                    } else {
                        Span::raw("")
                    },
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "  [Enter] Next/Submit  [Esc] Cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ];

            let popup = Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" Call Contract "),
            );
            f.render_widget(popup, area);
        }
        ActionMode::Result { message, success } => {
            let area = centered_rect(70, 7, f.area());
            f.render_widget(Clear, area);

            let (icon, color) = if *success {
                ("✓", Color::Green)
            } else {
                ("✗", Color::Red)
            };

            let lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(format!("  {icon} "), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                    Span::styled(message.as_str(), Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press any key to dismiss",
                    Style::default().fg(Color::DarkGray),
                )),
            ];

            let title = if *success { " Success " } else { " Error " };
            let popup = Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(color))
                    .title(title),
            );
            f.render_widget(popup, area);
        }
        ActionMode::None => {}
    }
}
