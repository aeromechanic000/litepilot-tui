//! Inline output rendering via `terminal.insert_before()`.
//!
//! Renders OutputLine variants into a ratatui Buffer for insertion above
//! the inline viewport. Content scrolls into the terminal's native scrollback.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use super::{OutputLine, theme::Theme};

/// Render a slice of OutputLine into the given buffer area for `insert_before()`.
pub fn render_output_lines(buf: &mut Buffer, lines: &[OutputLine], theme: &Theme) {
    let area = buf.area;
    let mut row: u16 = 0;

    for ol in lines {
        if row >= area.height {
            break;
        }
        let rendered = render_line(ol, theme);
        for line in rendered {
            if row >= area.height {
                break;
            }
            let line_rect = Rect::new(area.x, area.y + row, area.width, 1);
            let paragraph = ratatui::widgets::Paragraph::new(line);
            paragraph.render(line_rect, buf);
            row += 1;
        }
    }
}

/// Render a single OutputLine into one or more styled Lines.
fn render_line(ol: &OutputLine, theme: &Theme) -> Vec<Line<'static>> {
    match ol {
        OutputLine::User(text) => vec![Line::from(vec![
            Span::styled(
                "\u{25b6} ", // ▶
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                text.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ])],

        OutputLine::Assistant(text) => {
            if text.is_empty() {
                return vec![Line::from(Span::styled(
                    "\u{25cf} ", // ●
                    Style::default(),
                ))];
            }
            let mut lines = Vec::new();
            for (i, text_line) in text.lines().enumerate() {
                if i == 0 {
                    // First line gets the ● marker
                    lines.push(Line::from(vec![
                        Span::styled("\u{25cf} ", Style::default()),
                        Span::raw(text_line.to_string()),
                    ]));
                } else {
                    lines.push(Line::from(Span::raw(text_line.to_string())));
                }
            }
            if lines.is_empty() {
                lines.push(Line::from(Span::styled(
                    "\u{25cf} ",
                    Style::default(),
                )));
            }
            lines
        }

        OutputLine::System(text) => {
            let mut lines = Vec::new();
            for (i, line) in text.lines().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled(
                            "\u{203b} ", // ※
                            Style::default().fg(theme.accent),
                        ),
                        Span::styled(
                            line.to_string(),
                            Style::default().fg(Color::Reset),
                        ),
                    ]));
                } else {
                    lines.push(Line::from(Span::styled(
                        line.to_string(),
                        Style::default().fg(Color::Reset),
                    )));
                }
            }
            if lines.is_empty() {
                lines.push(Line::from(Span::styled(
                    "\u{203b} ".to_string(),
                    Style::default().fg(theme.accent),
                )));
            }
            lines
        }

        OutputLine::Error(text) => vec![Line::from(vec![
            Span::styled(
                "\u{2717} ", // ✗
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                text.clone(),
                Style::default().fg(theme.warning),
            ),
        ])],

        OutputLine::Code { language, code } => {
            let mut lines = Vec::new();
            lines.push(Line::from(Span::styled(
                format!("\u{250c}\u{2500} {} \u{2500}", language), // ┌─ lang ─
                Style::default().fg(theme.accent),
            )));
            for line in code.lines() {
                lines.push(Line::from(Span::raw(format!("  {}", line))));
            }
            lines.push(Line::from(Span::styled(
                "\u{2514}\u{2500}", // └─
                Style::default().fg(theme.accent),
            )));
            lines
        }

        OutputLine::Diff { added, removed } => {
            let mut lines = Vec::new();
            for r in removed {
                lines.push(Line::from(Span::styled(
                    format!("- {}", r),
                    Style::default().fg(Color::Red),
                )));
            }
            for a in added {
                lines.push(Line::from(Span::styled(
                    format!("+ {}", a),
                    Style::default().fg(Color::Green),
                )));
            }
            lines
        }

        OutputLine::Thinking(_) => vec![],

        OutputLine::Pending(text) => vec![Line::from(vec![
            Span::styled(
                "\u{25b8} ", // ▸
                Style::default().fg(theme.accent),
            ),
            Span::styled(
                format!("{} (queued)", text),
                Style::default().fg(theme.accent),
            ),
        ])],

        OutputLine::Plan(plan) => {
            let mut lines = vec![Line::from(Span::styled(
                "\u{25c6} Plan", // ◆
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))];
            for step in plan.lines() {
                let trimmed = step.trim();
                if !trimmed.is_empty() {
                    lines.push(Line::from(Span::raw(format!("  {}", trimmed))));
                }
            }
            lines
        }

        OutputLine::Separator => vec![Line::from(vec![
            Span::styled(
                "\u{2500}".repeat(20), // ────
                Style::default()
                    .fg(Color::DarkGray),
            ),
            Span::styled(
                " done ",
                Style::default()
                    .fg(Color::DarkGray),
            ),
            Span::styled(
                "\u{2500}".repeat(20),
                Style::default()
                    .fg(Color::DarkGray),
            ),
        ])],
    }
}

/// Estimate how many terminal rows a set of OutputLines will occupy when rendered.
/// Used as the `line_count` parameter for `insert_before()`.
pub fn estimate_line_count(lines: &[OutputLine], _width: u16) -> u16 {
    let mut count: u16 = 0;
    for ol in lines {
        match ol {
            OutputLine::User(text) => {
                count += 1;
                let _ = text;
            }
            OutputLine::Assistant(text) => {
                if text.is_empty() {
                    count += 1;
                } else {
                    // Approximate: number of newlines + 1
                    count += text.lines().count().max(1) as u16;
                }
            }
            OutputLine::System(text) => {
                count += text.lines().count().max(1) as u16;
            }
            OutputLine::Error(_) => count += 1,
            OutputLine::Code { code, .. } => {
                count += 2; // header + footer
                count += code.lines().count().max(1) as u16;
            }
            OutputLine::Diff { added, removed } => {
                count += (added.len() + removed.len()) as u16;
            }
            OutputLine::Thinking(_) => {} // not rendered inline
            OutputLine::Pending(_) => count += 1,
            OutputLine::Plan(plan) => {
                count += 1; // header
                count += plan.lines().filter(|l| !l.trim().is_empty()).count() as u16;
            }
            OutputLine::Separator => count += 1,
        }
    }
    count.max(1)
}
