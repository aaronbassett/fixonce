//! Dashboard view — the default landing screen.
//!
//! Layout:
//! ```text
//!   ┌─────────────────────────────────┐
//!   │  Hero Row (logo + info panel)   │  ← Length(8)
//!   ├─────────────────────────────────┤
//!   │  Activity Row                   │  ← Length(12)
//!   ├─────────────────────────────────┤
//!   │  Memory List                    │  ← Min(0)
//!   ├─────────────────────────────────┤
//!   │  Status Bar                     │  ← Length(1)
//!   └─────────────────────────────────┘
//! ```

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tui_big_text::{BigText, PixelSize};

use crate::tui::app::{App, View};

// Per-character rainbow colours for "FixOnce".
const LOGO_COLORS: [(u8, u8, u8); 7] = [
    (255, 107, 107), // F — red
    (255, 142, 107), // i — orange
    (255, 184, 107), // x — yellow-orange
    (107, 255, 107), // O — green
    (107, 196, 255), // n — cyan
    (107, 107, 255), // c — blue
    (184, 107, 255), // e — purple
];

/// Render the dashboard screen.
pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    // Outer vertical split: hero | activity | memory list | status bar.
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    render_hero_row(f, outer[0]);
    render_activity_row(f, outer[1]);
    render_memory_list(f, outer[2]);
    render_status_bar(f, app, outer[3]);
}

// ---------------------------------------------------------------------------
// Hero Row
// ---------------------------------------------------------------------------

fn render_hero_row(f: &mut Frame, area: Rect) {
    // Horizontal split: logo (66%) | info panel (34%).
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(66), Constraint::Percentage(34)])
        .split(area);

    render_logo(f, cols[0]);
    render_info_panel(f, cols[1]);
}

fn render_logo(f: &mut Frame, area: Rect) {
    let chars: Vec<Span> = "FixOnce"
        .chars()
        .enumerate()
        .map(|(i, ch)| {
            let (r, g, b) = LOGO_COLORS[i % LOGO_COLORS.len()];
            Span::styled(ch.to_string(), Style::default().fg(Color::Rgb(r, g, b)))
        })
        .collect();

    let big = BigText::builder()
        .pixel_size(PixelSize::Full)
        .lines(vec![Line::from(chars)])
        .alignment(Alignment::Left)
        .build();

    f.render_widget(big, area);
}

fn render_info_panel(f: &mut Frame, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let content = vec![
        Line::from(format!("Version: {version}")),
        Line::from(format!("OS: {os} {arch}")),
        Line::from("───────────────"),
        Line::from("Message of the Day"),
        Line::from(Span::styled(
            "\"Fix it once, remember it forever.\"",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::ITALIC),
        )),
    ];

    let info = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    f.render_widget(info, area);
}

// ---------------------------------------------------------------------------
// Activity Row (placeholder)
// ---------------------------------------------------------------------------

fn render_activity_row(f: &mut Frame, area: Rect) {
    let placeholder = Paragraph::new("Loading activity data...")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Activity "),
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));

    f.render_widget(placeholder, area);
}

// ---------------------------------------------------------------------------
// Memory List (placeholder)
// ---------------------------------------------------------------------------

fn render_memory_list(f: &mut Frame, area: Rect) {
    let placeholder = Paragraph::new("Loading memories...")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Memories "),
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));

    f.render_widget(placeholder, area);
}

// ---------------------------------------------------------------------------
// Status Bar
// ---------------------------------------------------------------------------

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");

    // Build the tab indicators.
    let tabs: &[(&str, bool)] = &[
        ("[1] Dashboard", matches!(app.current_view, View::Dashboard)),
        ("[2] Search", matches!(app.current_view, View::Search)),
        (
            "[3] Create",
            matches!(app.current_view, View::CreateForm),
        ),
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

    // Context hints.
    spans.push(Span::styled(
        "[/] graph  [;'] list  ↑↓ nav  Enter open",
        Style::default().fg(Color::DarkGray),
    ));

    // Right-aligned version string.
    let right_text = format!("fixonce v{version}");
    // We render the status bar as two Paragraphs: left and right, overlaid.
    // Simpler: build one line and pad.
    let left_line = Line::from(spans);
    let left = Paragraph::new(left_line).alignment(Alignment::Left);

    let right = Paragraph::new(Span::styled(
        right_text,
        Style::default().fg(Color::DarkGray),
    ))
    .alignment(Alignment::Right);

    f.render_widget(left, area);
    f.render_widget(right, area);
}
