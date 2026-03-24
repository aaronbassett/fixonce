//! Unauthenticated splash screen — shown when the user launches the TUI
//! without a valid auth token.
//!
//! The function is fully self-contained: it sets up its own terminal, renders
//! a single frame, waits for a keypress, then tears down the terminal before
//! returning.

use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Terminal,
};
use tui_big_text::{BigText, PixelSize};
use std::io;

// Rainbow gradient colours applied per-character across "FixOnce".
const RAINBOW: [Color; 7] = [
    Color::Rgb(255, 0, 0),   // red
    Color::Rgb(255, 127, 0), // orange
    Color::Rgb(255, 255, 0), // yellow
    Color::Rgb(0, 200, 0),   // green
    Color::Rgb(0, 210, 210), // cyan
    Color::Rgb(0, 0, 255),   // blue
    Color::Rgb(148, 0, 211), // purple
];

/// Display the unauthenticated splash screen and block until the user presses
/// any key.
///
/// This function manages its own terminal lifecycle (raw mode / alternate
/// screen) and is completely independent of the main TUI terminal.
pub fn show_unauthenticated_splash() -> Result<()> {
    // -----------------------------------------------------------------------
    // Terminal setup
    // -----------------------------------------------------------------------
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // -----------------------------------------------------------------------
    // Render a single frame
    // -----------------------------------------------------------------------
    terminal.draw(|f| {
        let area = f.area();

        // Split vertically: top padding | big-text logo | subtitle | bottom padding.
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Length(8),   // BigText::Full uses 8 rows
                Constraint::Length(2),   // subtitle
                Constraint::Min(0),
            ])
            .split(area);

        // Build the rainbow gradient BigText widget.
        // tui-big-text renders each Span in the Line with its own style, so
        // we create one Span per character with a cycling RAINBOW colour.
        let chars: Vec<Span> = "FixOnce"
            .chars()
            .enumerate()
            .map(|(i, ch)| {
                Span::styled(
                    ch.to_string(),
                    Style::default().fg(RAINBOW[i % RAINBOW.len()]),
                )
            })
            .collect();

        let big = BigText::builder()
            .pixel_size(PixelSize::Full)
            .lines(vec![Line::from(chars)])
            .alignment(Alignment::Center)
            .build();

        f.render_widget(big, chunks[1]);

        // Subtitle.
        let subtitle = Paragraph::new(
            "Exit and login with `fixonce login` before launching the TUI",
        )
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

        f.render_widget(subtitle, chunks[2]);
    })?;

    // -----------------------------------------------------------------------
    // Wait for any keypress
    // -----------------------------------------------------------------------
    loop {
        if let Ok(Event::Key(_)) = event::read() {
            break;
        }
    }

    // -----------------------------------------------------------------------
    // Terminal teardown
    // -----------------------------------------------------------------------
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        cursor::Show
    )?;

    Ok(())
}
