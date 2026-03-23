//! Dashboard view — the default landing screen.
//!
//! Layout:
//! ```text
//!   ┌─────────────────────────────────┐
//!   │  FixOnce  [search bar]          │  ← title + search
//!   ├─────────────────────────────────┤
//!   │  memory list (filtered)         │  ← centre
//!   ├─────────────────────────────────┤
//!   │  status bar                     │  ← bottom
//!   └─────────────────────────────────┘
//! ```

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::tui::app::App;

/// Render the dashboard screen.
pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    // Outer vertical split: header (3) | body | status (1).
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    // ---- Header / search bar ----
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(12), Constraint::Min(0)])
        .split(outer[0]);

    let title = Paragraph::new("  FixOnce")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, header_chunks[0]);

    let search_text = format!(" Search: {}_", app.search_query);
    let search = Paragraph::new(search_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title(" Filter "));
    f.render_widget(search, header_chunks[1]);

    // ---- Memory list ----
    let filtered = app.filtered_memories();
    let items: Vec<ListItem> = filtered
        .iter()
        .map(|m| {
            let decay_bar = decay_indicator(m.decay_score);
            let line = Line::from(vec![
                Span::styled(
                    format!(" {:>5.2} ", m.decay_score),
                    Style::default().fg(score_colour(m.decay_score)),
                ),
                Span::raw(decay_bar),
                Span::raw("  "),
                Span::styled(
                    format!("[{:<12}]", m.memory_type.to_string()),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw("  "),
                Span::raw(truncate(&m.title, 60)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(if filtered.is_empty() {
        None
    } else {
        Some(app.selected_index.min(filtered.len().saturating_sub(1)))
    });

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Memories ({}) ", filtered.len())),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, outer[1], &mut list_state);

    // ---- Status / keybinding bar ----
    let status_text = if let Some(ref msg) = app.status_message {
        msg.clone()
    } else {
        " [1]Dashboard  [2]List  [3]Create  [4]Activity  [5]Keys  [6]Secrets  [7]Health  [q]Quit"
            .to_owned()
    };
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    f.render_widget(status, outer[2]);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn decay_indicator(score: f64) -> &'static str {
    if score >= 90.0 {
        "████"
    } else if score >= 70.0 {
        "███░"
    } else if score >= 50.0 {
        "██░░"
    } else if score >= 30.0 {
        "█░░░"
    } else {
        "░░░░"
    }
}

fn score_colour(score: f64) -> Color {
    if score >= 80.0 {
        Color::Green
    } else if score >= 50.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_owned()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
