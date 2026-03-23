//! Activity stream view — recent activity log entries, auto-refresh via tick.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::App;

/// Render the activity stream screen.
pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // heading
            Constraint::Min(0),    // list
            Constraint::Length(1), // status
        ])
        .split(area);

    // ---- Heading ----
    let heading = Paragraph::new(" Activity Stream  (auto-refreshes every 5 s)")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(heading, chunks[0]);

    // ---- Activity list ----
    let visible_entries: Vec<&str> = app
        .activity_entries
        .iter()
        .skip(app.scroll_offset)
        .map(String::as_str)
        .collect();

    let items: Vec<ListItem> = if visible_entries.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            " No activity entries loaded.  (fetch happens on view open)",
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        visible_entries
            .iter()
            .map(|entry| {
                // Entries are expected to look like "2024-01-02T03:04:05Z  action  entity"
                ListItem::new(Line::from(Span::raw(format!("  {entry}"))))
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} Entries ", app.activity_entries.len())),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(list, chunks[1]);

    // ---- Status bar ----
    let status = Paragraph::new(" [↑↓ / j k] Scroll  [Esc] Back  [q] Quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    f.render_widget(status, chunks[2]);
}
