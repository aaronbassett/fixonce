//! Memory list view — filterable, sortable list of memories.
//!
//! Shows title, type, decay score, and last-updated timestamp.
//! Arrow keys navigate; Enter opens the detail view.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::tui::app::App;

/// Render the memory list screen.
pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // search bar
            Constraint::Min(0),    // list
            Constraint::Length(1), // status
        ])
        .split(area);

    // ---- Search bar ----
    let query_display = format!(" Filter: {}_", app.search_query);
    let search = Paragraph::new(query_display)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Memory List "),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(search, chunks[0]);

    // ---- Memory table ----
    let filtered = app.filtered_memories();

    let header_cells = ["Title", "Type", "Decay", "Last Updated"].iter().map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
    });
    let header = Row::new(header_cells).height(1).bottom_margin(0);

    let rows: Vec<Row> = filtered
        .iter()
        .map(|m| {
            let last_updated = m.updated_at.get(..10).unwrap_or(&m.updated_at).to_owned();
            let decay_str = format!("{:>5.1}", m.decay_score);
            let decay_style = if m.decay_score >= 80.0 {
                Style::default().fg(Color::Green)
            } else if m.decay_score >= 50.0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Red)
            };

            Row::new(vec![
                Cell::from(truncate(&m.title, 50)),
                Cell::from(m.memory_type.to_string()).style(Style::default().fg(Color::Yellow)),
                Cell::from(decay_str).style(decay_style),
                Cell::from(last_updated).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let mut table_state = ratatui::widgets::TableState::default();
    table_state.select(if filtered.is_empty() {
        None
    } else {
        Some(app.selected_index.min(filtered.len().saturating_sub(1)))
    });

    let table = Table::new(
        rows,
        [
            Constraint::Min(40),
            Constraint::Length(14),
            Constraint::Length(6),
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

    // ---- Status bar ----
    let status = if let Some(ref msg) = app.status_message {
        msg.clone()
    } else {
        " [↑↓] Navigate  [Enter] Detail  [Type] Filter  [Esc] Back  [q] Quit".to_owned()
    };
    let status_widget = Paragraph::new(status)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    f.render_widget(status_widget, chunks[2]);
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_owned()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
