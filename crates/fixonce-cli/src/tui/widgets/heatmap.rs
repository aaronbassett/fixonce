//! Activity heatmap widget — renders a 6-month calendar heatmap.

use std::collections::HashMap;

use fixonce_core::api::dashboard::HeatmapEntry;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::tui::app::HeatmapMode;

/// Color levels for the heatmap (GitHub-style green palette).
const LEVELS: [(Color, char); 5] = [
    (Color::Rgb(22, 42, 22), '░'),   // Level 0: no activity
    (Color::Rgb(14, 68, 41), '▒'),   // Level 1
    (Color::Rgb(0, 109, 50), '▓'),   // Level 2
    (Color::Rgb(38, 166, 65), '█'),  // Level 3
    (Color::Rgb(57, 211, 83), '█'),  // Level 4
];

/// Number of days in a given month/year.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Three-letter month abbreviation.
fn month_abbr(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

/// Parse an ISO date string ("2026-03-01") into (year, month, day).
fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;
    Some((year, month, day))
}

/// Get current year and month from the system clock.
fn current_year_month() -> (i32, u32) {
    // Use std::time to get a rough date.  We convert the UNIX timestamp into
    // a calendar date using basic arithmetic (no chrono / time crate needed).
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // Days since epoch (1970-01-01).
    let days = (secs / 86400) as i32;

    // Civil-date algorithm from Howard Hinnant (public domain).
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i32 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };

    (year, m)
}

/// Compute the last 6 (year, month) pairs ending at the given month.
fn last_6_months(year: i32, month: u32) -> Vec<(i32, u32)> {
    let mut months = Vec::with_capacity(6);
    let mut y = year;
    let mut m = month;
    for _ in 0..6 {
        months.push((y, m));
        if m == 1 {
            m = 12;
            y -= 1;
        } else {
            m -= 1;
        }
    }
    months.reverse(); // oldest first
    months
}

/// Render the activity heatmap into the given area.
///
/// This is a standalone render function (not a `Widget` impl) for simplicity.
pub fn render_heatmap(
    f: &mut Frame,
    area: Rect,
    entries: &[HeatmapEntry],
    mode: HeatmapMode,
) {
    let action = mode.action();

    // Build a map from (year, month, day) -> count for the selected action.
    let mut counts: HashMap<(i32, u32, u32), i64> = HashMap::new();
    for entry in entries {
        if entry.action != action {
            continue;
        }
        if let Some((y, m, d)) = parse_date(&entry.day) {
            *counts.entry((y, m, d)).or_default() += entry.count;
        }
    }

    // Find max count for level calculation.
    let max_count = counts.values().copied().max().unwrap_or(0).max(1);

    // Get the last 6 months.
    let (cur_year, cur_month) = current_year_month();
    let months = last_6_months(cur_year, cur_month);

    // Build lines for each month.
    let mut lines: Vec<Line<'static>> = Vec::new();

    for &(y, m) in &months {
        let max_day = days_in_month(y, m);
        let label = format!("{} ", month_abbr(m));
        let mut spans: Vec<Span<'static>> = vec![Span::styled(
            label,
            Style::default().fg(Color::DarkGray),
        )];

        for day in 1..=31u32 {
            if day > max_day {
                spans.push(Span::raw(" "));
            } else {
                let count = counts.get(&(y, m, day)).copied().unwrap_or(0);
                let level = if count == 0 {
                    0
                } else {
                    ((count * 4) / max_count).clamp(1, 4) as usize
                };
                let (color, ch) = LEVELS[level];
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(color),
                ));
            }
        }

        lines.push(Line::from(spans));
    }

    // Legend line — right-aligned via padding.
    let legend_text = "Less ░▒▓██ More";
    let legend_width = legend_text.chars().count() as u16;
    let avail = area.width.saturating_sub(1);
    let pad = avail.saturating_sub(legend_width) as usize;
    let legend_line = Line::from(vec![
        Span::raw(" ".repeat(pad)),
        Span::styled("Less ", Style::default().fg(Color::DarkGray)),
        Span::styled("░", Style::default().fg(LEVELS[0].0)),
        Span::styled("▒", Style::default().fg(LEVELS[1].0)),
        Span::styled("▓", Style::default().fg(LEVELS[2].0)),
        Span::styled("█", Style::default().fg(LEVELS[3].0)),
        Span::styled("█", Style::default().fg(LEVELS[4].0)),
        Span::styled(" More", Style::default().fg(Color::DarkGray)),
    ]);
    lines.push(legend_line);

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}
