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
    pub sidebar_visible: bool,
    pub sidebar_tab: SidebarTab,
    pub sidebar_scroll: u16,
    pub auto_scroll: bool,
    /// Accumulates streaming tokens into the last Assistant output line
    pub streaming_buffer: String,
    pub file_tree: Vec<FileEntry>,
    pub sidebar_selection: usize,
    pub workspace_hint: PathBuf,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub is_dir: bool,
    pub depth: usize,
    pub expanded: bool,
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
    #[allow(dead_code)]
    Thinking(String),
    Pending(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarTab {
    ProjectFiles,
    #[allow(dead_code)]
    CodeBase,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            input_text: String::new(),
            input_cursor: 0,
            output_lines: Vec::new(),
            scroll_offset: 0,
            sidebar_visible: true,
            sidebar_tab: SidebarTab::ProjectFiles,
            sidebar_scroll: 0,
            auto_scroll: true,
            streaming_buffer: String::new(),
            file_tree: Vec::new(),
            sidebar_selection: 0,
            workspace_hint: PathBuf::new(),
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
        self.input_text.insert(self.input_cursor, c);
        self.input_cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
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
        let input = self.input_text.clone();
        self.input_text.clear();
        self.input_cursor = 0;
        input
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

    pub fn set_file_tree(&mut self, entries: Vec<FileEntry>) {
        self.file_tree = entries;
        if self.sidebar_selection >= self.file_tree.len() && !self.file_tree.is_empty() {
            self.sidebar_selection = self.file_tree.len().saturating_sub(1);
        }
    }

    pub fn sidebar_move_up(&mut self) {
        if self.sidebar_selection > 0 {
            self.sidebar_selection -= 1;
        }
    }

    pub fn sidebar_move_down(&mut self) {
        if self.sidebar_selection + 1 < self.file_tree.len() {
            self.sidebar_selection += 1;
        }
    }

    #[allow(dead_code)]
    pub fn sidebar_toggle_expand(&mut self) {
        if self.sidebar_selection < self.file_tree.len() {
            self.file_tree[self.sidebar_selection].expanded =
                !self.file_tree[self.sidebar_selection].expanded;
        }
    }

    pub fn sidebar_switch_tab(&mut self) {
        self.sidebar_tab = match self.sidebar_tab {
            SidebarTab::ProjectFiles => SidebarTab::CodeBase,
            SidebarTab::CodeBase => SidebarTab::ProjectFiles,
        };
        self.sidebar_selection = 0;
        self.sidebar_scroll = 0;
    }
}

pub fn draw(f: &mut Frame, app: &AppState, ui: &mut UiState) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // status bar
            Constraint::Min(5),    // main area
            Constraint::Length(3), // input area
        ])
        .split(size);

    draw_status_bar(f, app, ui, chunks[0]);
    draw_main_area(f, app, ui, chunks[1]);
    draw_input_area(f, ui, chunks[2]);
}

fn draw_status_bar(f: &mut Frame, app: &AppState, ui: &UiState, area: Rect) {
    let theme = &ui.theme;
    let (mode_label, mode_color) = theme.mode_indicator(&app.mode);

    let search_indicator = if app.web_search_enabled {
        "SEARCH:ON"
    } else {
        "SEARCH:OFF"
    };
    let fast = truncate_model_name(app.config.effective_fast_model(), 12);
    let core = truncate_model_name(&app.config.core_model, 12);
    let audit = truncate_model_name(app.config.effective_audit_model(), 12);

    let mut spans = vec![
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
        Span::raw(" | "),
        Span::styled(
            format!("[{}]", mode_label),
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::raw(search_indicator),
        Span::raw(" | "),
        Span::raw(truncate_path(
            &app.workspace.to_string_lossy(),
            area.width as usize / 2,
        )),
    ];

    if app.is_processing {
        spans.push(Span::raw(" | "));
        spans.push(Span::styled(
            "thinking...",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
        if !app.pending_queue.is_empty() {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!("({} queued)", app.pending_queue.len()),
                Style::default().fg(theme.accent),
            ));
        }
    }

    let status = Paragraph::new(Line::from(spans));
    f.render_widget(status, area);
}

fn draw_main_area(f: &mut Frame, _app: &AppState, ui: &mut UiState, area: Rect) {
    let theme = &ui.theme;

    let (main_area, sidebar_area) = if ui.sidebar_visible && area.width > 60 {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
            .split(area);
        (split[0], Some(split[1]))
    } else {
        (area, None)
    };

    // Main output panel
    let lines: Vec<Line> = ui
        .output_lines
        .iter()
        .flat_map(|ol| match ol {
            OutputLine::User(text) => vec![Line::from(Span::styled(
                format!("> {}", text),
                Style::default().fg(theme.primary),
            ))],
            OutputLine::Assistant(text) => render_markdown(text, theme),
            OutputLine::System(text) => vec![Line::from(Span::styled(
                format!("[system] {}", text),
                Style::default().fg(theme.accent),
            ))],
            OutputLine::Error(text) => vec![Line::from(Span::styled(
                format!("[error] {}", text),
                Style::default().fg(theme.warning),
            ))],
            OutputLine::Code { language, code } => render_code_block(language, code, theme),
            OutputLine::Thinking(text) => vec![Line::from(Span::styled(
                format!("thinking: {}", text),
                Style::default().fg(theme.accent),
            ))],
            OutputLine::Pending(text) => vec![Line::from(Span::styled(
                format!("> {} (queued)", text),
                Style::default().fg(theme.accent),
            ))],
            OutputLine::Diff { added, removed } => render_diff(added, removed, theme),
        })
        .collect();

    let total_lines = lines.len() as u16;
    let visible_height = main_area.height;
    let max_scroll = total_lines.saturating_sub(visible_height);
    if ui.scroll_offset > max_scroll {
        ui.scroll_offset = max_scroll;
    }

    let output = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((ui.scroll_offset, 0));
    f.render_widget(output, main_area);

    // Sidebar
    if let Some(sidebar_area) = sidebar_area {
        draw_sidebar(f, ui, sidebar_area);
    }
}

fn draw_sidebar(f: &mut Frame, ui: &UiState, area: Rect) {
    let theme = &ui.theme;

    let tab_label = match ui.sidebar_tab {
        SidebarTab::ProjectFiles => "Project Files",
        SidebarTab::CodeBase => "Code Base",
    };

    let header = Line::from(Span::styled(
        format!(" {} ", tab_label),
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD),
    ));

    let mut lines: Vec<Line> = vec![header, Line::raw("")];

    match ui.sidebar_tab {
        SidebarTab::ProjectFiles => {
            if ui.file_tree.is_empty() {
                lines.push(Line::raw("  (empty)"));
            } else {
                let visible_height = area.height.saturating_sub(4) as usize;
                let start = ui.sidebar_scroll as usize;
                let end = std::cmp::min(start + visible_height, ui.file_tree.len());

                for (i, entry) in ui.file_tree[start..end].iter().enumerate() {
                    let actual_idx = start + i;
                    let is_selected = actual_idx == ui.sidebar_selection;

                    let indent = "  ".repeat(entry.depth);
                    let icon = if entry.is_dir {
                        if entry.expanded {
                            "v "
                        } else {
                            "> "
                        }
                    } else {
                        "  "
                    };
                    let name = entry.path.rsplit('/').next().unwrap_or(&entry.path);

                    let style = if is_selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(theme.primary)
                            .add_modifier(Modifier::BOLD)
                    } else if entry.is_dir {
                        Style::default().fg(theme.primary)
                    } else {
                        Style::default()
                    };

                    lines.push(Line::from(Span::styled(
                        format!("{}{}{}", indent, icon, name),
                        style,
                    )));
                }
            }
        }
        SidebarTab::CodeBase => {
            lines.push(Line::raw("  Built-in templates"));
            lines.push(Line::raw("  (see /skills)"));
        }
    }

    // Footer hint
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        " Tab:switch | Enter:toggle ",
        Style::default().fg(theme.accent),
    )));

    let sidebar = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(theme.primary)),
    );
    f.render_widget(sidebar, area);
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
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            if trimmed.starts_with("## ") {
                return Line::from(Span::styled(
                    trimmed.to_string(),
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            if trimmed.starts_with("# ") {
                return Line::from(Span::styled(
                    trimmed.to_string(),
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
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

fn render_inline_code<'a>(line: &'a str, theme: &'a Theme) -> Line<'a> {
    let mut spans = Vec::new();
    let mut in_code = false;
    let mut current = String::new();

    for ch in line.chars() {
        if ch == '`' {
            if !current.is_empty() {
                let style = if in_code {
                    Style::default().fg(theme.primary)
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
            Style::default().fg(theme.primary)
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

fn draw_input_area(f: &mut Frame, ui: &UiState, area: Rect) {
    let theme = &ui.theme;
    let input = Paragraph::new(ui.input_text.as_str()).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme.primary))
            .title(Span::styled(
                " Shift+Tab:mode | Enter:send | Ctrl+S:search | Ctrl+C:quit ",
                Style::default().fg(theme.accent),
            )),
    );
    f.render_widget(input, area);
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
    fn sidebar_tab_switch() {
        let mut ui = UiState::default();
        assert_eq!(ui.sidebar_tab, SidebarTab::ProjectFiles);
        ui.sidebar_switch_tab();
        assert_eq!(ui.sidebar_tab, SidebarTab::CodeBase);
        ui.sidebar_switch_tab();
        assert_eq!(ui.sidebar_tab, SidebarTab::ProjectFiles);
    }

    #[test]
    fn sidebar_navigation() {
        let mut ui = UiState::default();
        ui.file_tree = vec![
            FileEntry {
                path: "a".into(),
                is_dir: true,
                depth: 0,
                expanded: false,
            },
            FileEntry {
                path: "b".into(),
                is_dir: false,
                depth: 0,
                expanded: false,
            },
            FileEntry {
                path: "c".into(),
                is_dir: false,
                depth: 0,
                expanded: false,
            },
        ];
        assert_eq!(ui.sidebar_selection, 0);
        ui.sidebar_move_down();
        assert_eq!(ui.sidebar_selection, 1);
        ui.sidebar_move_down();
        assert_eq!(ui.sidebar_selection, 2);
        ui.sidebar_move_down(); // at end, no move
        assert_eq!(ui.sidebar_selection, 2);
        ui.sidebar_move_up();
        assert_eq!(ui.sidebar_selection, 1);
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
    fn sidebar_tab_default() {
        let ui = UiState::default();
        assert_eq!(ui.sidebar_tab, SidebarTab::ProjectFiles);
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
