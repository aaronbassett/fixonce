//! Health overview view — memory count, average scores, decay stats.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

use crate::tui::app::App;

/// Render the health overview screen.
pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // heading
            Constraint::Length(5), // summary stats
            Constraint::Length(5), // decay gauge
            Constraint::Min(0),    // padding
            Constraint::Length(1), // status bar
        ])
        .split(area);

    // ---- Heading ----
    let heading = Paragraph::new(" Health Overview")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(heading, chunks[0]);

    // ---- Summary stats ----
    let memories = &app.memories;
    let total = memories.len();
    // Allow precision loss: we are computing display-only averages,
    // not financial arithmetic.
    #[allow(clippy::cast_precision_loss)]
    let avg_decay = if total == 0 {
        0.0_f64
    } else {
        memories.iter().map(|m| m.decay_score).sum::<f64>() / total as f64
    };
    #[allow(clippy::cast_precision_loss)]
    let avg_reinforcement = if total == 0 {
        0.0_f64
    } else {
        memories.iter().map(|m| m.reinforcement_score).sum::<f64>() / total as f64
    };
    let low_decay_count = memories.iter().filter(|m| m.decay_score < 50.0).count();

    let stats_lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled(
                "  Total Memories        : ",
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(total.to_string()),
        ]),
        Line::from(vec![
            Span::styled(
                "  Avg Decay Score       : ",
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!("{avg_decay:.2}"),
                Style::default().fg(score_colour(avg_decay)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Avg Reinforcement     : ",
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(format!("{avg_reinforcement:.2}")),
        ]),
        Line::from(vec![
            Span::styled(
                "  Low Decay (<50)       : ",
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!("{low_decay_count}"),
                if low_decay_count > 0 {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::Green)
                },
            ),
        ]),
    ];

    let stats_widget =
        Paragraph::new(stats_lines).block(Block::default().borders(Borders::ALL).title(" Stats "));
    f.render_widget(stats_widget, chunks[1]);

    // ---- Decay gauge ----
    let gauge_ratio = (avg_decay / 100.0).clamp(0.0, 1.0);
    let decay_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Avg Decay Score "),
        )
        .gauge_style(
            Style::default()
                .fg(score_colour(avg_decay))
                .bg(Color::DarkGray),
        )
        .ratio(gauge_ratio)
        .label(format!("{avg_decay:.1}%"));
    f.render_widget(decay_gauge, chunks[2]);

    // ---- Status bar ----
    let status_bar = Paragraph::new(" [Esc] Back  [q] Quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    f.render_widget(status_bar, chunks[4]);
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
