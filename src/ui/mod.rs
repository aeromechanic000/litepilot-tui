pub mod theme;

use crate::app::AppState;
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub struct UiState {
    pub theme: Theme,
    pub input_text: String,
    pub input_cursor: usize,
    pub output_lines: Vec<OutputLine>,
    pub scroll_offset: u16,
    pub sidebar_visible: bool,
    pub sidebar_tab: SidebarTab,
}

#[derive(Debug, Clone)]
pub enum OutputLine {
    User(String),
    Assistant(String),
    System(String),
    Error(String),
    #[allow(dead_code)]
    Code { language: String, code: String },
    #[allow(dead_code)]
    Diff { added: Vec<String>, removed: Vec<String> },
    #[allow(dead_code)]
    Thinking(String),
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
}

impl UiState {
    pub fn add_output(&mut self, line: OutputLine) {
        self.output_lines.push(line);
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
}

pub fn draw(f: &mut Frame, app: &AppState, ui: &mut UiState) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // status bar
            Constraint::Min(5),     // main area
            Constraint::Length(3),  // input area
        ])
        .split(size);

    draw_status_bar(f, app, ui, chunks[0]);
    draw_main_area(f, app, ui, chunks[1]);
    draw_input_area(f, ui, chunks[2]);
}

fn draw_status_bar(f: &mut Frame, app: &AppState, ui: &UiState, area: Rect) {
    let theme = &ui.theme;
    let (mode_label, mode_color) = theme.mode_indicator(&app.mode);

    let search_indicator = if app.web_search_enabled { "SEARCH:ON" } else { "SEARCH:OFF" };
    let fast = truncate_model_name(&app.config.effective_fast_model(), 12);
    let core = truncate_model_name(&app.config.core_model, 12);
    let audit = truncate_model_name(&app.config.effective_audit_model(), 12);

    let spans = vec![
        Span::styled(" LiteCode ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        Span::styled(" | ", Style::default().fg(theme.text)),
        Span::styled(&app.config.ollama_endpoint, Style::default().fg(theme.text)),
        Span::styled(" | ", Style::default().fg(theme.text)),
        Span::styled(format!("F:{}", fast), Style::default().fg(theme.text)),
        Span::styled(" ", Style::default()),
        Span::styled(format!("C:{}", core), Style::default().fg(theme.text)),
        Span::styled(" ", Style::default()),
        Span::styled(format!("A:{}", audit), Style::default().fg(theme.text)),
        Span::styled(" | ", Style::default().fg(theme.text)),
        Span::styled(format!("[{}]", mode_label), Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
        Span::styled(" | ", Style::default().fg(theme.text)),
        Span::styled(search_indicator, Style::default().fg(theme.text)),
        Span::styled(" | ", Style::default().fg(theme.text)),
        Span::styled(
            truncate_path(&app.workspace.to_string_lossy(), area.width as usize / 2),
            Style::default().fg(theme.text),
        ),
    ];

    let status = Paragraph::new(Line::from(spans))
        .style(Style::default().fg(theme.text).bg(theme.bg_main));
    f.render_widget(status, area);
}

fn draw_main_area(f: &mut Frame, _app: &AppState, ui: &mut UiState, area: Rect) {
    let theme = &ui.theme;

    let (main_area, sidebar_area) = if ui.sidebar_visible && area.width > 60 {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(75),
                Constraint::Percentage(25),
            ])
            .split(area);
        (split[0], Some(split[1]))
    } else {
        (area, None)
    };

    // Main output panel
    let lines: Vec<Line> = ui.output_lines.iter().map(|ol| {
        match ol {
            OutputLine::User(text) => Line::from(Span::styled(
                format!("> {}", text),
                Style::default().fg(theme.primary),
            )),
            OutputLine::Assistant(text) => Line::from(Span::styled(
                text,
                Style::default().fg(theme.text),
            )),
            OutputLine::System(text) => Line::from(Span::styled(
                format!("[system] {}", text),
                Style::default().fg(theme.thinking),
            )),
            OutputLine::Error(text) => Line::from(Span::styled(
                format!("[error] {}", text),
                Style::default().fg(theme.error),
            )),
            OutputLine::Code { language, code } => {
                let header = format!("[{}]", language);
                let content = code.lines().take(50).map(|l| format!("  {}", l)).collect::<Vec<_>>().join("\n");
                Line::from(Span::styled(
                    format!("{}\n{}", header, content),
                    Style::default().fg(theme.code_keyword),
                ))
            }
            OutputLine::Thinking(text) => Line::from(Span::styled(
                format!("thinking: {}", text),
                Style::default().fg(theme.thinking),
            )),
            OutputLine::Diff { added, removed } => {
                let mut parts = Vec::new();
                for r in removed {
                    parts.push(format!("- {}", r));
                }
                for a in added {
                    parts.push(format!("+ {}", a));
                }
                Line::from(Span::styled(
                    parts.join("\n"),
                    Style::default().fg(theme.text),
                ))
            }
        }
    }).collect();

    // Clear main area background first
    let clear = Block::default()
        .style(Style::default().bg(theme.bg_main));
    f.render_widget(clear, main_area);

    let output = Paragraph::new(lines)
        .style(Style::default().fg(theme.text).bg(theme.bg_main))
        .wrap(Wrap { trim: false })
        .scroll((ui.scroll_offset, 0));
    f.render_widget(output, main_area);

    // Sidebar
    if let Some(sidebar_area) = sidebar_area {
        let tab_indicator = match ui.sidebar_tab {
            SidebarTab::ProjectFiles => "Project Files",
            SidebarTab::CodeBase => "Code Base",
        };
        // Clear the entire sidebar area first to prevent leftover colors
        let clear = Block::default()
            .style(Style::default().fg(theme.text).bg(theme.bg_sidebar));
        f.render_widget(clear, sidebar_area);
        // Then render the content on top
        let sidebar = Paragraph::new(tab_indicator)
            .style(Style::default().fg(theme.text).bg(theme.bg_sidebar))
            .block(Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(theme.primary))
            );
        f.render_widget(sidebar, sidebar_area);
    }
}

fn draw_input_area(f: &mut Frame, ui: &UiState, area: Rect) {
    let theme = &ui.theme;
    let input = Paragraph::new(ui.input_text.as_str())
        .style(Style::default().fg(theme.text).bg(theme.bg_main))
        .block(Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme.primary))
            .title(Span::styled(
                " Shift+Tab:mode | Enter:send | Ctrl+S:search | Ctrl+C:quit ",
                Style::default().fg(theme.text),
            ))
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
    fn truncate_model() {
        assert_eq!(truncate_model_name("qwen3:4b", 12), "qwen3:4b");
        assert_eq!(truncate_model_name("very-long-model-name:72b", 12), "very-long-..");
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
}
