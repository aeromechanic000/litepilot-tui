pub mod theme;

use crate::app::AppState;
use crate::ui::theme::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use std::path::PathBuf;

pub struct UiState {
    pub theme: Theme,
    pub input_text: String,
    pub input_cursor: usize,
    pub output_lines: Vec<OutputLine>,
    pub scroll_offset: u16,
    pub auto_scroll: bool,
    /// Accumulates streaming tokens into the last Assistant output line
    pub streaming_buffer: String,
    pub workspace_hint: PathBuf,
    /// Tick counter for animating the thinking indicator dots
    pub thinking_tick: u8,
    /// Stores full multi-line paste content; input_text shows the summary
    paste_buffer: Option<String>,
    /// Remembers the last user input for plan approval flow
    pub last_user_input: String,
}

#[derive(Debug, Clone)]
pub enum OutputLine {
    User(String),
    Assistant(String),
    System(String),
    Error(String),
    #[allow(dead_code)]
    Code {
        language: String,
        code: String,
    },
    #[allow(dead_code)]
    Diff {
        added: Vec<String>,
        removed: Vec<String>,
    },
    Thinking(()),
    Pending(String),
    Plan(String),
    Separator,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            input_text: String::new(),
            input_cursor: 0,
            output_lines: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
            streaming_buffer: String::new(),
            workspace_hint: PathBuf::new(),
            thinking_tick: 0,
            paste_buffer: None,
            last_user_input: String::new(),
        }
    }
}

impl UiState {
    pub fn from_config(config: &crate::config::Config) -> Self {
        Self {
            theme: Theme::from_config(&config.theme),
            ..Self::default()
        }
    }

    pub fn with_workspace(mut self, workspace: PathBuf) -> Self {
        self.workspace_hint = workspace;
        self
    }
}

impl UiState {
    pub fn add_output(&mut self, line: OutputLine) {
        self.output_lines.push(line);
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Append a streaming chunk to the last Assistant output line.
    /// Creates a new Assistant line if none exists or if the last isn't Assistant.
    pub fn append_stream_chunk(&mut self, chunk: &str) {
        self.streaming_buffer.push_str(chunk);
        match self.output_lines.last_mut() {
            Some(OutputLine::Assistant(existing)) => {
                existing.push_str(chunk);
            }
            _ => {
                self.output_lines
                    .push(OutputLine::Assistant(chunk.to_string()));
            }
        }
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Add a Thinking indicator line below the last user message.
    pub fn start_thinking(&mut self) {
        self.output_lines.push(OutputLine::Thinking(()));
        self.thinking_tick = 0;
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Remove the last Thinking line (if present) before replacing with response.
    /// Returns true if a Thinking line was removed.
    pub fn stop_thinking(&mut self) -> bool {
        if matches!(self.output_lines.last(), Some(OutputLine::Thinking(()))) {
            self.output_lines.pop();
            true
        } else {
            false
        }
    }

    /// Finish streaming — finalizes the buffer. Returns the full content.
    pub fn finish_stream(&mut self) -> String {
        let content = std::mem::take(&mut self.streaming_buffer);
        // If streaming produced nothing, add an empty Assistant line
        if content.is_empty()
            && !self
                .output_lines
                .last()
                .is_some_and(|l| matches!(l, OutputLine::Assistant(_)))
        {
            self.add_output(OutputLine::Assistant(String::new()));
        }
        content
    }

    #[allow(dead_code)]
    pub fn clear_output(&mut self) {
        self.output_lines.clear();
        self.scroll_offset = 0;
    }

    pub fn push_char(&mut self, c: char) {
        self.paste_buffer = None;
        self.input_text.insert(self.input_cursor, c);
        self.input_cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.paste_buffer.is_some() {
            self.paste_buffer = None;
            self.input_text.clear();
            self.input_cursor = 0;
            return;
        }
        if self.input_cursor > 0 {
            let prev = self.input_text[..self.input_cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.input_cursor -= prev;
            self.input_text.remove(self.input_cursor);
        }
    }

    pub fn take_input(&mut self) -> String {
        let input = self.paste_buffer.take().unwrap_or_else(|| self.input_text.clone());
        self.input_text.clear();
        self.input_cursor = 0;
        input
    }

    /// Handle pasted text. Multi-line pastes show a summary in the input field
    /// while storing the full content for submission.
    pub fn set_paste(&mut self, text: String) {
        let line_count = text.lines().count();
        if line_count > 1 {
            self.paste_buffer = Some(text);
            self.input_text = format!("[{} copied lines]", line_count);
            self.input_cursor = self.input_text.len();
        } else {
            self.paste_buffer = None;
            for c in text.chars() {
                self.input_text.insert(self.input_cursor, c);
                self.input_cursor += c.len_utf8();
            }
        }
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
        // scroll_offset will be clamped during render
        self.scroll_offset = u16::MAX;
    }
}

pub fn draw(f: &mut Frame, app: &AppState, ui: &mut UiState) {
    let size = f.area();

    // Calculate dynamic input height: grows with content, capped at 40% of terminal
    let input_content_lines = estimate_input_lines(&ui.input_text, size.width);
    let max_input = (size.height / 5).max(4).min(15); // 20% of terminal, min 4, max 15
    let input_height = (input_content_lines + 1).max(3).min(max_input);

    // Status bar is always 2 lines
    let status_bar_height: u16 = 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(status_bar_height), // status bar (wraps if needed)
            Constraint::Min(5),                    // main area
            Constraint::Length(input_height),      // input area
        ])
        .split(size);

    draw_status_bar(f, app, ui, chunks[0]);
    draw_main_area(f, app, ui, chunks[1]);

    let visible_content_rows = chunks[2].height.saturating_sub(1) as usize; // -1 for border
    let has_overflow = input_content_lines as usize > visible_content_rows;
    draw_input_area(f, ui, chunks[2], has_overflow);

    // Place visible cursor in the input area
    set_input_cursor(f, ui, chunks[2]);
}

fn think_mode_label(enabled: &bool) -> &'static str {
    if *enabled { "THINK" } else { "DIRECT" }
}

fn draw_status_bar(f: &mut Frame, app: &AppState, ui: &UiState, area: Rect) {
    let theme = &ui.theme;
    let (mode_label, mode_color) = theme.mode_indicator(&app.mode);
    let think_label = think_mode_label(&app.think_enabled);
    let fast = truncate_model_name(app.config.effective_fast_model(), 12);
    let core = truncate_model_name(&app.config.core_model, 12);
    let audit = truncate_model_name(app.config.effective_audit_model(), 12);

    let think_color = if app.think_enabled { theme.accent } else { theme.warning };

    // Line 1: LitePilot | endpoint | models
    let line1 = Line::from(vec![
        Span::styled(
            " LitePilot ",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::raw(&app.config.ollama_endpoint),
        Span::raw(" | "),
        Span::raw(format!("F:{}", fast)),
        Span::raw(" "),
        Span::raw(format!("C:{}", core)),
        Span::raw(" "),
        Span::raw(format!("A:{}", audit)),
    ]);

    // Line 2: mode | think | workspace
    let mut line2_spans = vec![
        Span::styled(
            format!(" [{}] ", mode_label),
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw("| "),
        Span::styled(
            format!("[{}]", think_label),
            Style::default().fg(think_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::raw(truncate_path(
            &app.workspace.to_string_lossy(),
            area.width as usize / 2,
        )),
    ];

    if !app.pending_queue.is_empty() {
        line2_spans.push(Span::raw(" | "));
        line2_spans.push(Span::styled(
            format!("({} queued)", app.pending_queue.len()),
            Style::default().fg(theme.accent),
        ));
    }

    let line2 = Line::from(line2_spans);

    let status = Paragraph::new(vec![line1, line2]);
    f.render_widget(status, area);
}

fn draw_main_area(f: &mut Frame, _app: &AppState, ui: &mut UiState, area: Rect) {
    let theme = &ui.theme;

    // Animate thinking dots when a Thinking line is present
    let has_thinking = ui
        .output_lines
        .iter()
        .any(|ol| matches!(ol, OutputLine::Thinking(_)));
    if has_thinking {
        ui.thinking_tick = ui.thinking_tick.wrapping_add(1);
    }
    let tick = ui.thinking_tick;

    // Main output panel
    let lines: Vec<Line> = ui
        .output_lines
        .iter()
        .flat_map(|ol| match ol {
            OutputLine::User(text) => vec![Line::from(vec![
                Span::styled(
                    "\u{25b6} ", // ▶ RIGHT-POINTING TRIANGLE
                    Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
                ),
                Span::styled(text.clone(), Style::default().add_modifier(Modifier::BOLD)),
            ])],
            OutputLine::Assistant(text) => {
                let mut rendered = render_markdown(text, theme);
                if let Some(first) = rendered.first_mut() {
                    // Prepend ● BLACK CIRCLE marker by rebuilding the line
                    let old = std::mem::take(first);
                    let marker = Span::styled(
                        "\u{25cf} ", // ● BLACK CIRCLE
                        Style::default(),
                    );
                    let mut new_spans = vec![marker];
                    new_spans.extend(old.spans);
                    *first = Line::from(new_spans);
                }
                rendered
            }
            OutputLine::System(text) => vec![Line::from(vec![
                Span::styled(
                    "\u{203b} ", // ※ REFERENCE MARK
                    Style::default().fg(theme.accent),
                ),
                Span::styled(text.clone(), Style::default().fg(Color::Reset)),
            ])],
            OutputLine::Error(text) => vec![Line::from(vec![
                Span::styled(
                    "\u{2717} ", // ✗ BALLOT X
                    Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
                ),
                Span::styled(text.clone(), Style::default().fg(theme.warning)),
            ])],
            OutputLine::Code { language, code } => render_code_block(language, code, theme),
            OutputLine::Thinking(_) => {
                let dots = ".".repeat(((tick / 4) % 3 + 1) as usize);
                vec![Line::from(Span::styled(
                    format!("\u{2234} thinking{}", dots), // ∴ THEREFORE
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::ITALIC),
                ))]
            }
            OutputLine::Pending(text) => vec![Line::from(vec![
                Span::styled(
                    "\u{25b8} ", // ▸ SMALL RIGHT-POINTING TRIANGLE
                    Style::default().fg(theme.accent),
                ),
                Span::styled(
                    format!("{} (queued)", text),
                    Style::default().fg(theme.accent),
                ),
            ])],
            OutputLine::Plan(plan) => {
                let mut lines = vec![Line::from(Span::styled(
                    "\u{25c6} Plan".to_string(), // ◆ BLACK DIAMOND
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ))];
                for step in plan.lines() {
                    let trimmed = step.trim();
                    if !trimmed.is_empty() {
                        lines.push(Line::from(Span::styled(
                            format!("  {}", trimmed),
                            Style::default().fg(Color::Reset),
                        )));
                    }
                }
                lines
            }
            OutputLine::Diff { added, removed } => render_diff(added, removed, theme),
            OutputLine::Separator => vec![Line::from(vec![
                Span::styled(
                    "\u{2500}".repeat(20), // ────────
                    Style::default().fg(theme.accent).add_modifier(Modifier::DIM),
                ),
                Span::styled(
                    " done ",
                    Style::default().fg(theme.accent).add_modifier(Modifier::DIM),
                ),
                Span::styled(
                    "\u{2500}".repeat(20),
                    Style::default().fg(theme.accent).add_modifier(Modifier::DIM),
                ),
            ])],
        })
        .collect();

    let total_lines = lines.len() as u16;
    let visible_height = area.height;
    let max_scroll = total_lines.saturating_sub(visible_height);
    if ui.scroll_offset > max_scroll {
        ui.scroll_offset = max_scroll;
    }

    let output = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((ui.scroll_offset, 0));
    f.render_widget(output, area);
}

/// Render assistant text with basic markdown formatting.
fn render_markdown<'a>(text: &'a str, theme: &'a Theme) -> Vec<Line<'a>> {
    text.lines()
        .map(|line| {
            let trimmed = line.trim();

            // Headers
            if trimmed.starts_with("### ") {
                return Line::from(Span::styled(
                    trimmed.to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            }
            if trimmed.starts_with("## ") {
                return Line::from(Span::styled(
                    trimmed.to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            }
            if trimmed.starts_with("# ") {
                return Line::from(Span::styled(
                    trimmed.to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            }

            // Bullet points
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("• ")
            {
                return Line::from(Span::raw(line.to_string()));
            }

            // Numbered lists
            if let Some(rest) = trimmed
                .strip_prefix("1. ")
                .or_else(|| trimmed.strip_prefix("2. "))
                .or_else(|| trimmed.strip_prefix("3. "))
                .or_else(|| trimmed.strip_prefix("4. "))
                .or_else(|| trimmed.strip_prefix("5. "))
            {
                let _ = rest;
                return Line::from(Span::raw(line.to_string()));
            }

            // Inline code (simple: just color backtick-enclosed segments)
            if line.contains('`') {
                return render_inline_code(line, theme);
            }

            Line::raw(line.to_string())
        })
        .collect()
}

fn render_inline_code<'a>(line: &'a str, _theme: &'a Theme) -> Line<'a> {
    let mut spans = Vec::new();
    let mut in_code = false;
    let mut current = String::new();

    for ch in line.chars() {
        if ch == '`' {
            if !current.is_empty() {
                let style = if in_code {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                spans.push(Span::styled(std::mem::take(&mut current), style));
            }
            in_code = !in_code;
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        let style = if in_code {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        spans.push(Span::styled(current, style));
    }

    Line::from(spans)
}

/// Render a code block with syntax highlighting.
fn render_code_block<'a>(language: &'a str, code: &str, theme: &'a Theme) -> Vec<Line<'a>> {
    let mut lines = Vec::new();

    // Header line
    lines.push(Line::from(Span::styled(
        format!("┌─ {} ─", language),
        Style::default().fg(theme.accent),
    )));

    // Try syntect highlighting
    let highlighted = highlight_code(code, language);

    for line in highlighted {
        lines.push(line);
    }

    // Footer
    lines.push(Line::from(Span::styled(
        "└─",
        Style::default().fg(theme.accent),
    )));

    lines
}

fn highlight_code<'a>(code: &str, language: &str) -> Vec<Line<'a>> {
    let ss = syntect::parsing::SyntaxSet::load_defaults_newlines();
    let ts = syntect::highlighting::ThemeSet::load_defaults();
    let theme = &ts.themes["base16-eighties.dark"];

    let syntax = ss
        .find_syntax_by_token(language)
        .or_else(|| ss.find_syntax_by_extension(language))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut rng = syntect::easy::HighlightLines::new(syntax, theme);
    let mut result = Vec::new();

    for line in syntect::util::LinesWithEndings::from(code) {
        let ranges: Vec<(syntect::highlighting::Style, &str)> =
            rng.highlight_line(line, &ss).unwrap_or_default();
        let spans: Vec<Span> = ranges
            .iter()
            .map(|(style, text): &(_, _)| {
                let color = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                let mut modifiers = Modifier::empty();
                if style
                    .font_style
                    .contains(syntect::highlighting::FontStyle::BOLD)
                {
                    modifiers |= Modifier::BOLD;
                }
                if style
                    .font_style
                    .contains(syntect::highlighting::FontStyle::ITALIC)
                {
                    modifiers |= Modifier::ITALIC;
                }
                Span::styled(
                    text.to_string(),
                    Style::default().fg(color).add_modifier(modifiers),
                )
            })
            .collect();
        result.push(Line::from(spans));
    }

    result
}

/// Render diff with colored +/- lines.
fn render_diff<'a>(added: &[String], removed: &[String], _theme: &'a Theme) -> Vec<Line<'a>> {
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

fn draw_input_area(f: &mut Frame, ui: &UiState, area: Rect, has_overflow: bool) {
    let theme = &ui.theme;
    let text_width = area.width.saturating_sub(3) as usize; // " > " takes 3 cols
    let visible_rows = area.height.saturating_sub(1) as usize; // -1 for border

    let wrapped = wrap_input_text(&ui.input_text, text_width);

    let mut para_lines: Vec<Line> = Vec::new();

    // Determine how many lines we can actually show
    let show_count = if has_overflow {
        visible_rows.saturating_sub(1) // reserve last row for overflow indicator
    } else {
        wrapped.len()
    };

    for (i, chunk) in wrapped.iter().enumerate() {
        if i >= show_count {
            break;
        }
        if i == 0 {
            para_lines.push(Line::from(vec![
                Span::styled(" > ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(chunk.clone()),
            ]));
        } else {
            para_lines.push(Line::from(vec![
                Span::raw("   "),
                Span::raw(chunk.clone()),
            ]));
        }
    }

    if has_overflow {
        let hidden = wrapped.len() - show_count;
        para_lines.push(Line::from(Span::styled(
            format!("   ... {} more line(s)", hidden),
            Style::default().add_modifier(Modifier::DIM),
        )));
    }

    let input = Paragraph::new(para_lines).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme.primary))
            .title(Span::styled(
                " Shift+Tab:mode | Enter:send | Shift+Enter:↵ | Ctrl+Tab:think | Ctrl+C:quit ",
                Style::default().fg(theme.accent),
            )),
    );
    f.render_widget(input, area);
}

/// Position the terminal cursor at the current input cursor location.
fn set_input_cursor(f: &mut Frame, ui: &UiState, area: Rect) {
    let text_width = area.width.saturating_sub(3) as usize; // " > " takes 3 cols
    if text_width == 0 {
        return;
    }

    // Count characters before the cursor position
    let chars_before: usize = ui.input_text[..ui.input_cursor]
        .chars()
        .count();

    // The first line has a 3-char " > " prefix, continuation lines have "   "
    let prefix = 3u16;
    let line = (chars_before / text_width) as u16;
    let col = (chars_before % text_width) as u16;

    let x = area.x + prefix + col;
    let y = area.y + 1 + line; // +1 for the top border

    if y < area.y + area.height {
        f.set_cursor_position((x, y));
    }
}

/// Break text into lines that fit within `width` characters.
fn wrap_input_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        current.push(ch);
        if current.chars().count() >= width {
            lines.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }

    lines
}

/// Estimate how many content lines the input text will occupy when wrapped.
fn estimate_input_lines(text: &str, area_width: u16) -> u16 {
    if area_width <= 3 {
        return 1;
    }
    let text_width = (area_width - 3) as usize;
    if text_width == 0 {
        return 1;
    }
    let char_count = text.chars().count();
    ((char_count + text_width - 1) / text_width).max(1) as u16
}

fn truncate_model_name(name: &str, max: usize) -> String {
    if name.len() <= max {
        name.to_string()
    } else {
        format!("{}..", &name[..max.saturating_sub(2)])
    }
}

fn truncate_path(path: &str, max: usize) -> String {
    if path.len() <= max {
        path.to_string()
    } else {
        format!("...{}", &path[path.len() - max + 3..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_state_push_and_take_input() {
        let mut ui = UiState::default();
        for c in "hello".chars() {
            ui.push_char(c);
        }
        assert_eq!(ui.input_text, "hello");
        let taken = ui.take_input();
        assert_eq!(taken, "hello");
        assert!(ui.input_text.is_empty());
    }

    #[test]
    fn ui_state_backspace() {
        let mut ui = UiState::default();
        for c in "ab".chars() {
            ui.push_char(c);
        }
        ui.backspace();
        assert_eq!(ui.input_text, "a");
        ui.backspace();
        assert_eq!(ui.input_text, "");
        ui.backspace(); // no-op on empty
        assert_eq!(ui.input_text, "");
    }

    #[test]
    fn ui_state_add_output() {
        let mut ui = UiState::default();
        ui.add_output(OutputLine::User("test".into()));
        ui.add_output(OutputLine::Assistant("response".into()));
        assert_eq!(ui.output_lines.len(), 2);
    }

    #[test]
    fn scroll_up_disables_auto_scroll() {
        let mut ui = UiState::default();
        assert!(ui.auto_scroll);
        ui.scroll_up(5);
        assert!(!ui.auto_scroll);
    }

    #[test]
    fn add_output_respects_scroll_state() {
        let mut ui = UiState::default();
        // Default is auto_scroll=true, so add_output scrolls to bottom
        ui.add_output(OutputLine::User("test".into()));
        assert!(ui.auto_scroll);
        // After scrolling up, new output should not auto-scroll
        ui.scroll_up(1);
        assert!(!ui.auto_scroll);
        ui.add_output(OutputLine::User("test2".into()));
        assert!(!ui.auto_scroll); // stays false
    }

    #[test]
    fn truncate_model() {
        assert_eq!(truncate_model_name("qwen3:4b", 12), "qwen3:4b");
        assert_eq!(
            truncate_model_name("very-long-model-name:72b", 12),
            "very-long-.."
        );
    }

    #[test]
    fn truncate_path_fn() {
        assert_eq!(truncate_path("/short", 10), "/short");
        let long = "/very/long/path/to/some/file";
        let truncated = truncate_path(long, 15);
        assert!(truncated.starts_with("..."));
        assert!(truncated.len() <= 15);
    }

    #[test]
    fn render_markdown_headers() {
        let theme = Theme::default();
        let lines = render_markdown("# Hello\n## World\n### Sub", &theme);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn render_diff_colors() {
        let theme = Theme::default();
        let lines = render_diff(&["added line".into()], &["removed line".into()], &theme);
        assert_eq!(lines.len(), 2);
    }
}
