//! Search view — full-text, vector, and hybrid memory search.
//!
//! Layout:
//! ```text
//!   ┌──────────────────────────────────┐
//!   │  Search bar + type pills         │  ← Length(3)
//!   ├──────────────────────────────────┤
//!   │  Results list                    │  ← Min(0)
//!   ├──────────────────────────────────┤
//!   │  Status bar                      │  ← Length(1)
//!   └──────────────────────────────────┘
//! ```

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::tui::app::{App, InputMode, SearchType, View};
use crate::tui::data::DataState;

/// Render the search screen.
pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    // Outer vertical split: search bar | results | status bar.
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    render_search_bar_row(f, app, outer[0]);
    render_results(f, app, outer[1]);
    render_status_bar(f, app, outer[2]);
}

// ---------------------------------------------------------------------------
// Search bar row
// ---------------------------------------------------------------------------

fn render_search_bar_row(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(30)])
        .split(area);

    render_search_input(f, app, cols[0]);
    render_type_pills(f, app, cols[1]);
}

fn render_search_input(f: &mut Frame, app: &App, area: Rect) {
    let is_input = app.input_mode == InputMode::Input;
    let border_color = if is_input {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let mut display_text = app.search_query.clone();
    if is_input {
        display_text.push('_');
    }

    let input = Paragraph::new(display_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Search ")
                .border_style(Style::default().fg(border_color)),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(input, area);
}

fn render_type_pills(f: &mut Frame, app: &App, area: Rect) {
    let pills = [
        ("Hybrid", SearchType::Hybrid),
        ("FTS", SearchType::Fts),
        ("Vector", SearchType::Vector),
    ];

    let spans: Vec<Span> = pills
        .iter()
        .flat_map(|(label, st)| {
            let style = if *st == app.search_type {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::Gray).bg(Color::DarkGray)
            };
            vec![Span::styled(format!(" {label} "), style), Span::raw(" ")]
        })
        .collect();

    let line1 = Line::from(spans);
    let line2 = Line::from(Span::styled(
        "  Tab to switch type",
        Style::default().fg(Color::DarkGray),
    ));

    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Render the two lines inside the block.
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    f.render_widget(Paragraph::new(line1), rows[0]);
    f.render_widget(Paragraph::new(line2), rows[1]);
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

fn render_results(f: &mut Frame, app: &App, area: Rect) {
    match &app.search_results {
        DataState::Loading => {
            if app.search_query.is_empty() {
                // No search has been done yet.
                let hint = Paragraph::new(Span::styled(
                    "Type a query and press Enter to search",
                    Style::default().fg(Color::Gray),
                ))
                .alignment(Alignment::Center);
                f.render_widget(hint, area);
            } else {
                let loading = Paragraph::new(Span::styled(
                    "Searching...",
                    Style::default().fg(Color::Cyan),
                ))
                .alignment(Alignment::Center);
                f.render_widget(loading, area);
            }
        }
        DataState::Loaded(resp) => {
            if resp.hits.is_empty() {
                let empty = Paragraph::new(Span::styled(
                    "No results found",
                    Style::default().fg(Color::DarkGray),
                ))
                .alignment(Alignment::Center);
                f.render_widget(empty, area);
            } else {
                render_results_list(f, app, area, resp);
            }
        }
        DataState::Error(e) => {
            let err = Paragraph::new(Span::styled(
                format!("Error: {e}"),
                Style::default().fg(Color::Red),
            ))
            .alignment(Alignment::Center);
            f.render_widget(err, area);
        }
    }
}

fn render_results_list(
    f: &mut Frame,
    app: &App,
    area: Rect,
    resp: &fixonce_core::memory::types::SearchMemoryResponse,
) {
    let items: Vec<ListItem> = resp
        .hits
        .iter()
        .map(|hit| {
            let mem = &hit.memory;
            let type_str = mem.memory_type.to_string();
            let type_color = memory_type_color(&mem.memory_type);
            let badge = Span::styled(format!("[{type_str}]"), Style::default().fg(type_color));
            let title = truncate(mem.title.as_str(), 50);
            let score = format!("{:.4}", hit.similarity);

            let line1 = Line::from(vec![
                badge,
                Span::raw("  "),
                Span::styled(title, Style::default().fg(Color::White)),
                Span::raw("  "),
                Span::styled(score, Style::default().fg(Color::Cyan)),
            ]);

            let summary = truncate(mem.summary.as_str(), area.width.saturating_sub(4) as usize);
            let line2 = Line::from(vec![
                Span::raw("  "),
                Span::styled(summary, Style::default().fg(Color::DarkGray)),
            ]);

            ListItem::new(vec![line1, line2])
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::Rgb(42, 42, 74)))
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(Some(app.selected_index));
    f.render_stateful_widget(list, area, &mut state);
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");

    let tabs: &[(&str, bool)] = &[
        ("[1] Dashboard", matches!(app.current_view, View::Dashboard)),
        ("[2] Search", matches!(app.current_view, View::Search)),
        ("[3] Create", matches!(app.current_view, View::CreateForm)),
        ("[4] Keys", matches!(app.current_view, View::Keys)),
        ("[q] Quit", false),
    ];

    let mut spans: Vec<Span> = Vec::new();
    for (label, active) in tabs {
        let style = if *active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(label.to_string(), style));
        spans.push(Span::raw("  "));
    }

    // Context hints for the search view.
    spans.push(Span::styled(
        "[/] search  ↑↓ nav  Enter open  Tab type",
        Style::default().fg(Color::DarkGray),
    ));

    let left_line = Line::from(spans);
    let left = Paragraph::new(left_line).alignment(Alignment::Left);

    let right = Paragraph::new(Span::styled(
        format!("fixonce v{version}"),
        Style::default().fg(Color::DarkGray),
    ))
    .alignment(Alignment::Right);

    f.render_widget(left, area);
    f.render_widget(right, area);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn memory_type_color(mt: &fixonce_core::memory::types::MemoryType) -> Color {
    use fixonce_core::memory::types::MemoryType;
    match mt {
        MemoryType::Gotcha => Color::Yellow,
        MemoryType::BestPractice => Color::Rgb(255, 165, 0),
        MemoryType::Correction => Color::Cyan,
        MemoryType::AntiPattern => Color::Red,
        MemoryType::Discovery => Color::Green,
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_owned()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}
