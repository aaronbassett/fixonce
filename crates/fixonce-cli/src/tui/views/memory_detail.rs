//! Memory detail view — full content, metadata, scores, scrollable.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, View};

/// Render the memory detail screen.
#[allow(clippy::too_many_lines)]
pub fn render(f: &mut Frame, app: &App) {
    let id = match &app.current_view {
        View::MemoryDetail(id) => id.clone(),
        _ => return,
    };

    let memory = app.memories.iter().find(|m| m.id == id);

    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // status
        ])
        .split(area);

    // ---- Title bar ----
    let title_text = memory.map_or_else(
        || " Memory Detail ".to_owned(),
        |m| format!(" {} ", m.title),
    );
    let title_bar = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title_bar, chunks[0]);

    // ---- Main content (scrollable) ----
    if let Some(m) = memory {
        let mut lines: Vec<Line> = Vec::new();

        // Metadata section.
        push_kv(&mut lines, "ID", &m.id);
        push_kv(&mut lines, "Type", &m.memory_type.to_string());
        push_kv(&mut lines, "Source", &m.source_type.to_string());
        if let Some(ref lang) = m.language {
            push_kv(&mut lines, "Language", lang);
        }
        push_kv(&mut lines, "Decay Score", &format!("{:.2}", m.decay_score));
        push_kv(
            &mut lines,
            "Reinforcement",
            &format!("{:.2}", m.reinforcement_score),
        );
        push_kv(&mut lines, "Pipeline", &m.pipeline_status.to_string());
        push_kv(&mut lines, "Embedding", &m.embedding_status.to_string());
        push_kv(&mut lines, "Created At", &m.created_at);
        push_kv(&mut lines, "Updated At", &m.updated_at);
        if let Some(ref acc) = m.last_accessed_at {
            push_kv(&mut lines, "Last Accessed", acc);
        }
        push_kv(&mut lines, "Created By", &m.created_by);

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "── Summary ─────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )));
        for l in m.summary.lines() {
            lines.push(Line::from(Span::raw(l.to_owned())));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "── Content ─────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )));
        for l in m.content.lines() {
            lines.push(Line::from(Span::raw(l.to_owned())));
        }

        // Provenance extras.
        let extras: Vec<(&str, &Option<String>)> = vec![
            ("Source URL", &m.source_url),
            ("Repo URL", &m.repo_url),
            ("Session", &m.session_id),
            ("Task Summary", &m.task_summary),
        ];
        let any_extras = extras.iter().any(|(_, v)| v.is_some());
        if any_extras {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "── Provenance ──────────────────────────",
                Style::default().fg(Color::DarkGray),
            )));
            for (label, val) in &extras {
                if let Some(v) = val {
                    push_kv(&mut lines, label, v);
                }
            }
        }

        // Cast is safe: scroll_offset is bounded by the frame height which is
        // well within u16 range in practice.
        #[allow(clippy::cast_possible_truncation)]
        let scroll = app.scroll_offset as u16;
        let content = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));
        f.render_widget(content, chunks[1]);
    } else {
        let not_found = Paragraph::new("Memory not found.")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Red));
        f.render_widget(not_found, chunks[1]);
    }

    // ---- Status bar ----
    let status_bar = Paragraph::new(" [↑↓ / j k] Scroll  [Esc / Backspace] Back  [q] Quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    f.render_widget(status_bar, chunks[2]);
}

fn push_kv(lines: &mut Vec<Line<'_>>, key: &str, value: &str) {
    lines.push(Line::from(vec![
        Span::styled(
            format!("{key:<18}: "),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(value.to_owned()),
    ]));
}
