//! Create-memory form view.
//!
//! Fields: title, content, summary, type, source, language.
//! Tab / Shift-Tab move between fields; Ctrl+S shows a submit hint.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, FormField};

/// Render the create-memory form.
pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // heading
            Constraint::Length(3), // title
            Constraint::Length(5), // content
            Constraint::Length(4), // summary
            Constraint::Length(3), // type + source (side by side)
            Constraint::Length(3), // language
            Constraint::Min(0),    // padding
            Constraint::Length(1), // status
        ])
        .split(area);

    // ---- Heading ----
    let heading = Paragraph::new(" Create Memory")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(heading, chunks[0]);

    // ---- Title field ----
    render_field(
        f,
        chunks[1],
        "Title",
        &app.form_title,
        app.form_field == FormField::Title,
    );

    // ---- Content field ----
    render_field(
        f,
        chunks[2],
        "Content",
        &app.form_content,
        app.form_field == FormField::Content,
    );

    // ---- Summary field ----
    render_field(
        f,
        chunks[3],
        "Summary",
        &app.form_summary,
        app.form_field == FormField::Summary,
    );

    // ---- Type + Source (split horizontally) ----
    let type_source = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[4]);

    render_field(
        f,
        type_source[0],
        "Type (gotcha|best_practice|correction|anti_pattern|discovery)",
        &app.form_memory_type,
        app.form_field == FormField::MemoryType,
    );
    render_field(
        f,
        type_source[1],
        "Source (correction|observation|pr_feedback|manual|harvested)",
        &app.form_source,
        app.form_field == FormField::Source,
    );

    // ---- Language field ----
    render_field(
        f,
        chunks[5],
        "Language (optional)",
        &app.form_language,
        app.form_field == FormField::Language,
    );

    // ---- Status bar ----
    let status_text = if let Some(ref msg) = app.status_message {
        msg.clone()
    } else {
        " [Tab] Next Field  [Shift+Tab] Prev  [Ctrl+S] Submit Hint  [Esc] Cancel  [q] Quit"
            .to_owned()
    };
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    f.render_widget(status, chunks[7]);
}

fn render_field(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    label: &str,
    value: &str,
    active: bool,
) {
    let style = if active {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Gray)
    };
    let border_style = if active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let display = format!("{value}_");
    let widget = Paragraph::new(display)
        .style(style)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(format!(" {label} ")),
        );
    f.render_widget(widget, area);
}
