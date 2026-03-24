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
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use tui_big_text::{BigText, PixelSize};

use crate::tui::app::{App, ListMode, View};
use fixonce_core::memory::types::MemoryType;

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
    render_activity_row(f, app, outer[1]);
    render_memory_list(f, app, outer[2]);
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
// Activity Row
// ---------------------------------------------------------------------------

fn render_activity_row(f: &mut Frame, app: &App, area: Rect) {
    // Horizontal split: heatmap (50%) | stats (50%).
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_heatmap_panel(f, app, cols[0]);
    render_stats_panel(f, app, cols[1]);
}

fn render_heatmap_panel(f: &mut Frame, app: &App, area: Rect) {
    let title = format!(" {} ", app.heatmap_mode.label());
    let hint = " [ ] switch [ ] ";
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_bottom(Line::from(hint).alignment(Alignment::Right));

    let inner = block.inner(area);
    f.render_widget(block, area);

    match app.dashboard_data.as_loaded() {
        Some(data) => {
            crate::tui::widgets::heatmap::render_heatmap(
                f,
                inner,
                &data.heatmap,
                app.heatmap_mode,
            );
        }
        None => {
            let loading = Paragraph::new("Loading heatmap...")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(loading, inner);
        }
    }
}

fn render_stats_panel(f: &mut Frame, app: &App, area: Rect) {
    // Vertical split: total memories (flex:1) | bottom row (Length:5).
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(5)])
        .split(area);

    render_total_memories(f, app, rows[0]);
    render_stats_bottom_row(f, app, rows[1]);
}

fn render_total_memories(f: &mut Frame, app: &App, area: Rect) {
    let count = app
        .dashboard_data
        .as_loaded()
        .map(|d| d.stats.total_memories)
        .unwrap_or(0);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Total Memories ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.dashboard_data.as_loaded().is_some() {
        let big = BigText::builder()
            .pixel_size(PixelSize::HalfHeight)
            .style(Style::default().fg(Color::Rgb(107, 255, 107)))
            .lines(vec![Line::from(format!("{count}"))])
            .build();
        f.render_widget(big, inner);
    } else {
        let loading = Paragraph::new("—")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(loading, inner);
    }
}

fn render_stats_bottom_row(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Searches 24h
    let searches = app
        .dashboard_data
        .as_loaded()
        .map(|d| d.stats.searches_24h.to_string())
        .unwrap_or_else(|| "—".to_owned());

    let searches_widget = Paragraph::new(Span::styled(
        searches,
        Style::default()
            .fg(Color::Rgb(107, 196, 255))
            .add_modifier(Modifier::BOLD),
    ))
    .block(Block::default().borders(Borders::ALL).title(" Searches 24h "))
    .alignment(Alignment::Center);

    f.render_widget(searches_widget, cols[0]);

    // Reports 24h
    let reports = app
        .dashboard_data
        .as_loaded()
        .map(|d| d.stats.reports_24h.to_string())
        .unwrap_or_else(|| "—".to_owned());

    let reports_widget = Paragraph::new(Span::styled(
        reports,
        Style::default()
            .fg(Color::Rgb(255, 107, 107))
            .add_modifier(Modifier::BOLD),
    ))
    .block(Block::default().borders(Borders::ALL).title(" Reports 24h "))
    .alignment(Alignment::Center);

    f.render_widget(reports_widget, cols[1]);
}

// ---------------------------------------------------------------------------
// Memory List
// ---------------------------------------------------------------------------

fn render_memory_list(f: &mut Frame, app: &App, area: Rect) {
    // Vertical split: header (1) | list (Min) | footer (1).
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    // Header line.
    let header_left = Span::styled(
        format!(" {} ", app.list_mode.label()),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    let header_right = Span::styled(
        "[;] prev  ['] next ",
        Style::default().fg(Color::DarkGray),
    );
    let header_line = Line::from(vec![header_left, header_right]);
    let header = Paragraph::new(header_line).alignment(Alignment::Left);
    f.render_widget(header, rows[0]);

    // Build list items based on current mode.
    let items: Vec<ListItem> = build_list_items(app);
    let empty_state = get_empty_state(app);

    let list_widget = if items.is_empty() {
        let empty = Paragraph::new(Span::styled(
            empty_state,
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Center);
        f.render_widget(empty, rows[1]);
        // Footer hint.
        let footer = Paragraph::new(Span::styled(
            " ↑↓ navigate  Enter open  max 20 shown",
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Left);
        f.render_widget(footer, rows[2]);
        return;
    } else {
        List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 40, 60))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ")
    };

    let mut state = ListState::default();
    state.select(Some(app.selected_index));
    f.render_stateful_widget(list_widget, rows[1], &mut state);

    // Footer hint.
    let footer = Paragraph::new(Span::styled(
        " ↑↓ navigate  Enter open  max 20 shown",
        Style::default().fg(Color::DarkGray),
    ))
    .alignment(Alignment::Left);
    f.render_widget(footer, rows[2]);
}

fn memory_type_color(memory_type_str: &str) -> Color {
    match memory_type_str {
        "gotcha" => Color::Yellow,
        "best_practice" => Color::Rgb(255, 165, 0), // orange
        "correction" => Color::Cyan,
        "anti_pattern" => Color::Red,
        "discovery" => Color::Green,
        _ => Color::White,
    }
}

fn memory_type_color_from_enum(mt: &MemoryType) -> Color {
    match mt {
        MemoryType::Gotcha => Color::Yellow,
        MemoryType::BestPractice => Color::Rgb(255, 165, 0),
        MemoryType::Correction => Color::Cyan,
        MemoryType::AntiPattern => Color::Red,
        MemoryType::Discovery => Color::Green,
    }
}

fn truncate_title(title: &str, max_len: usize) -> String {
    if title.len() <= max_len {
        title.to_owned()
    } else {
        format!("{}…", &title[..max_len.saturating_sub(1)])
    }
}

fn build_list_items(app: &App) -> Vec<ListItem<'static>> {
    match app.list_mode {
        ListMode::RecentlyCreated => {
            app.memories
                .iter()
                .take(20)
                .map(|m| {
                    let type_str = m.memory_type.to_string();
                    let color = memory_type_color_from_enum(&m.memory_type);
                    let badge = Span::styled(
                        format!("[{type_str}]"),
                        Style::default().fg(color),
                    );
                    let title = truncate_title(&m.title, 40);
                    let decay = format!("  {:.2}", m.decay_score);
                    let line = Line::from(vec![
                        badge,
                        Span::raw(" "),
                        Span::styled(title, Style::default().fg(Color::White)),
                        Span::styled(decay, Style::default().fg(Color::DarkGray)),
                    ]);
                    ListItem::new(line)
                })
                .collect()
        }
        ListMode::RecentlyViewed => {
            let views = app
                .dashboard_data
                .as_loaded()
                .map(|d| d.recent_views.as_slice())
                .unwrap_or(&[]);
            views
                .iter()
                .take(20)
                .map(|rv| {
                    let color = memory_type_color(&rv.memory_type);
                    let badge = Span::styled(
                        format!("[{}]", rv.memory_type),
                        Style::default().fg(color),
                    );
                    let title = truncate_title(&rv.title, 35);
                    let viewed = format!("  last viewed: {}", rv.last_viewed);
                    let line = Line::from(vec![
                        badge,
                        Span::raw(" "),
                        Span::styled(title, Style::default().fg(Color::White)),
                        Span::styled(viewed, Style::default().fg(Color::DarkGray)),
                    ]);
                    ListItem::new(line)
                })
                .collect()
        }
        ListMode::MostAccessed => {
            let accessed = app
                .dashboard_data
                .as_loaded()
                .map(|d| d.most_accessed.as_slice())
                .unwrap_or(&[]);
            accessed
                .iter()
                .take(20)
                .map(|ma| {
                    let color = memory_type_color(&ma.memory_type);
                    let badge = Span::styled(
                        format!("[{}]", ma.memory_type),
                        Style::default().fg(color),
                    );
                    let title = truncate_title(&ma.title, 35);
                    let count = format!("  accessed: {}x", ma.access_count);
                    let line = Line::from(vec![
                        badge,
                        Span::raw(" "),
                        Span::styled(title, Style::default().fg(Color::White)),
                        Span::styled(count, Style::default().fg(Color::DarkGray)),
                    ]);
                    ListItem::new(line)
                })
                .collect()
        }
    }
}

fn get_empty_state(app: &App) -> &'static str {
    match app.list_mode {
        ListMode::RecentlyCreated => "No memories yet",
        ListMode::RecentlyViewed => "No views recorded",
        ListMode::MostAccessed => "No access data",
    }
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
