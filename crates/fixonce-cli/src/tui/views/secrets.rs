//! Secret management view — admin-only view to list/create secrets.
//!
//! This view intentionally does NOT display secret values.
//! Creation is delegated to the CLI (`fixonce` sub-command planned).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::App;

/// Render the secrets management screen.
pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // heading
            Constraint::Min(0),    // body
            Constraint::Length(1), // status
        ])
        .split(area);

    // ---- Heading ----
    let heading = Paragraph::new(" Secret Management  (admin only)")
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(heading, chunks[0]);

    // ---- Body ----
    let body_text = vec![
        "",
        "  Secret values are NEVER displayed in the TUI.",
        "",
        "  Available operations (via CLI):",
        "",
        "    fixonce secret create <name>   — store a new secret (admin)",
        "    fixonce secret get <name>      — retrieve a secret value (admin)",
        "",
        "  The TUI can show whether secrets exist.  To create or rotate",
        "  secrets, use the CLI commands above or the Supabase dashboard.",
        "",
    ];

    let body = Paragraph::new(body_text.join("\n"))
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(body, chunks[1]);

    // ---- Status bar ----
    let status_text = if let Some(ref msg) = app.status_message {
        msg.clone()
    } else {
        " [Esc] Back  [q] Quit".to_owned()
    };
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    f.render_widget(status, chunks[2]);
}
