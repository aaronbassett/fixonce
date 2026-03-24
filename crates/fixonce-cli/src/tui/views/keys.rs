//! Key management view — list registered Ed25519 keys; hint to revoke via CLI.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::tui::app::App;

/// A key entry as we display it in the TUI.
#[derive(Debug, Clone)]
pub struct KeyEntry {
    pub id: String,
    pub label: String,
    pub public_key_truncated: String,
    pub last_used_at: Option<String>,
    pub created_at: String,
}

/// Render the key management screen.
#[allow(clippy::too_many_lines)]
pub fn render(f: &mut Frame, app: &App) {
    // We re-use `app.activity_entries` to carry serialised key entries when
    // the event loop fetches them.  Each entry is a tab-separated record:
    //   id\tlabel\tpublic_key\tlast_used\tcreated
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // heading
            Constraint::Min(0),    // table
            Constraint::Length(3), // revoke hint
            Constraint::Length(1), // status
        ])
        .split(area);

    // ---- Heading ----
    let heading = Paragraph::new(" Registered Signing Keys")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(heading, chunks[0]);

    // ---- Keys table ----
    // Entries are stored as tab-separated fields in app.activity_entries
    // when the Keys view is active (see event-loop fetch logic).
    let keys: Vec<KeyEntry> = app
        .activity_entries
        .iter()
        .filter_map(|line| parse_key_entry(line))
        .collect();

    let header = Row::new(vec![
        Cell::from("ID").style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ),
        Cell::from("Label").style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ),
        Cell::from("Public Key").style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ),
        Cell::from("Last Used").style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ),
        Cell::from("Created").style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ),
    ])
    .height(1);

    let rows: Vec<Row> = if keys.is_empty() {
        vec![Row::new(vec![
            Cell::from("  No keys loaded. Run `fixonce keys list` to see keys."),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
        ])
        .style(Style::default().fg(Color::DarkGray))]
    } else {
        keys.iter()
            .map(|k| {
                Row::new(vec![
                    Cell::from(truncate(&k.id, 8)),
                    Cell::from(truncate(&k.label, 20)),
                    Cell::from(k.public_key_truncated.clone()),
                    Cell::from(
                        k.last_used_at
                            .as_deref()
                            .and_then(|s| s.get(..10))
                            .unwrap_or("—")
                            .to_owned(),
                    )
                    .style(Style::default().fg(Color::DarkGray)),
                    Cell::from(k.created_at.get(..10).unwrap_or(&k.created_at).to_owned())
                        .style(Style::default().fg(Color::DarkGray)),
                ])
            })
            .collect()
    };

    let mut table_state = TableState::default();
    table_state.select(if keys.is_empty() {
        None
    } else {
        Some(app.selected_index.min(keys.len().saturating_sub(1)))
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(22),
            Constraint::Length(20),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL))
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("> ");

    f.render_stateful_widget(table, chunks[1], &mut table_state);

    // ---- Revoke hint ----
    let hint = Paragraph::new(
        "  To revoke a key, run:\n  \
         fixonce keys revoke <key-id>",
    )
    .style(Style::default().fg(Color::Yellow))
    .block(Block::default().borders(Borders::ALL).title(" Hint "));
    f.render_widget(hint, chunks[2]);

    // ---- Status bar ----
    let status = Paragraph::new(" [↑↓] Navigate  [Esc] Back  [q] Quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    f.render_widget(status, chunks[3]);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_key_entry(line: &str) -> Option<KeyEntry> {
    let parts: Vec<&str> = line.splitn(5, '\t').collect();
    if parts.len() < 5 {
        return None;
    }
    Some(KeyEntry {
        id: parts[0].to_owned(),
        label: parts[1].to_owned(),
        public_key_truncated: parts[2].to_owned(),
        last_used_at: if parts[3].is_empty() || parts[3] == "null" {
            None
        } else {
            Some(parts[3].to_owned())
        },
        created_at: parts[4].to_owned(),
    })
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_owned()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
