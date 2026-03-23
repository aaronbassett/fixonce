//! Minimum-size warning view — shown when the terminal is too small (EC-35).

use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::{MIN_COLS, MIN_ROWS};

/// Render the "terminal too small" warning.
pub fn render(f: &mut Frame, area: Rect) {
    let msg = format!(
        "\n  Terminal is too small.\n\n  \
         Minimum size: {MIN_COLS} columns × {MIN_ROWS} rows.\n  \
         Current size: {} × {}.\n\n  \
         Resize the terminal or press [q] to quit.",
        area.width, area.height,
    );
    let widget = Paragraph::new(msg)
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::ALL).title(" FixOnce "));
    f.render_widget(widget, area);
}
