mod agent;
mod app;
mod codebase;
mod config;
mod ollama;
mod project;
mod sandbox;
mod search;
mod session;
mod skills;
mod ui;
mod util;
mod wizard;

use anyhow::Result;
use app::{AppMode, AppState};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use ui::OutputLine;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "litecode", version, about = "Terminal AI coding assistant powered by local Ollama models")]
struct Args {
    /// Working directory (defaults to current directory)
    #[arg(short, long)]
    dir: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Determine workspace
    let workspace = match args.dir {
        Some(ref d) => PathBuf::from(d),
        None => std::env::current_dir()?,
    };

    // Setup config — use project-local .litecode if present, else global ~/.litecode
    let litecode_dir = config::Config::ensure_dirs_for(&workspace)?;
    let config = config::Config::load_for_workspace(&workspace).unwrap_or_else(|_| {
        let default = config::Config::default();
        let _ = default.save(&litecode_dir.join("config.toml"));
        default
    });

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run setup wizard — user confirms or changes Ollama URL and model selection
    let config = wizard::run(&mut terminal, config, &workspace)?;

    // Run app
    let result = run_app(&mut terminal, config, workspace);

    // Restore terminal
    disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: config::Config,
    workspace: PathBuf,
) -> Result<()> {
    let mut app_state = AppState::new(config, workspace);
    let mut ui_state = ui::UiState::default();

    // Welcome message
    ui_state.add_output(OutputLine::System(
        "Welcome to LiteCode! Ollama-powered local coding agent.".into(),
    ));
    ui_state.add_output(OutputLine::System(
        "Shift+Tab: switch mode | Enter: send | Ctrl+C: quit | /skills: list skills".into(),
    ));

    // Show loaded skills count
    let skill_count = app_state.skills.list().len();
    if skill_count > 0 {
        let names: Vec<&str> = app_state.skills.list().iter().map(|s| s.name.as_str()).collect();
        ui_state.add_output(OutputLine::System(
            format!("Loaded {} skills: /{}", skill_count, names.join(", /")),
        ));
    }

    let ollama_client = ollama::OllamaClient::new(&app_state.config)?;

    // Check Ollama connectivity in background
    let endpoint = app_state.config.ollama_endpoint.clone();
    let ping_result = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let client = ollama::OllamaClient::new(&config::Config {
                ollama_endpoint: endpoint,
                ..config::Config::default()
            })?;
            client.ping().await
        })
    });

    if let Ok(Ok(_)) = ping_result.join() {
        ui_state.add_output(OutputLine::System(
            format!("Connected to Ollama at {}", app_state.config.ollama_endpoint),
        ));
    } else {
        ui_state.add_output(OutputLine::Error(
            format!("Cannot connect to Ollama at {}. Start Ollama first.", app_state.config.ollama_endpoint),
        ));
    }

    loop {
        terminal.draw(|f| ui::draw(f, &app_state, &mut ui_state))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match (key.modifiers, key.code) {
                    // Shift+Tab: switch mode
                    (KeyModifiers::SHIFT, KeyCode::Tab) => {
                        let new_mode = app_state.switch_mode();
                        ui_state.add_output(OutputLine::System(
                            format!("Switched to {} mode", new_mode),
                        ));
                    }
                    // Ctrl+C: quit
                    (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                        if app_state.mode == AppMode::Auto {
                            ui_state.add_output(OutputLine::System(
                                "Press Ctrl+C again to confirm quit from AUTO mode".into(),
                            ));
                            // Simple double-press: check next event
                            if event::poll(Duration::from_secs(2))? {
                                if let Event::Key(k2) = event::read()? {
                                    if k2.code == KeyCode::Char('c') && k2.modifiers.contains(KeyModifiers::CONTROL) {
                                        break;
                                    }
                                }
                            }
                        } else {
                            break;
                        }
                    }
                    // Ctrl+S: toggle search
                    (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                        app_state.web_search_enabled = !app_state.web_search_enabled;
                        let state = if app_state.web_search_enabled { "ON" } else { "OFF" };
                        ui_state.add_output(OutputLine::System(
                            format!("Web search: {}", state),
                        ));
                    }
                    // Enter: submit input
                    (KeyModifiers::NONE, KeyCode::Enter) => {
                        let input = ui_state.take_input();
                        if !input.is_empty() {
                            app_state.input_history.push(input.clone());
                            ui_state.add_output(OutputLine::User(input.clone()));

                            if input.trim() == "/quit" || input.trim() == "/exit" {
                                break;
                            }

                            // Handle slash commands
                            let trimmed = input.trim();
                            if trimmed == "/skills" {
                                let skills = app_state.skills.list();
                                if skills.is_empty() {
                                    ui_state.add_output(OutputLine::System("No skills loaded.".into()));
                                } else {
                                    let mut lines = vec!["Available skills:".to_string()];
                                    for s in skills {
                                        lines.push(format!("  /{} — {}", s.name, s.description));
                                    }
                                    ui_state.add_output(OutputLine::System(lines.join("\n")));
                                }
                            } else if trimmed == "/setup" {
                                match wizard::run(terminal, app_state.config.clone(), &app_state.workspace) {
                                    Ok(new_config) => {
                                        app_state.config = new_config;
                                        ui_state.add_output(OutputLine::System(
                                            "Setup complete. Configuration updated.".into(),
                                        ));
                                    }
                                    Err(e) => {
                                        ui_state.add_output(OutputLine::Error(
                                            format!("Setup wizard failed: {}", e),
                                        ));
                                    }
                                }
                                let skills = app_state.skills.list();
                                if skills.is_empty() {
                                    ui_state.add_output(OutputLine::System("No skills loaded.".into()));
                                } else {
                                    let mut lines = vec!["Available skills:".to_string()];
                                    for s in skills {
                                        lines.push(format!("  /{} — {}", s.name, s.description));
                                    }
                                    ui_state.add_output(OutputLine::System(lines.join("\n")));
                                }
                            } else if let Some(cmd) = trimmed.strip_prefix('/') {
                                let (skill_name, args) = match cmd.split_once(' ') {
                                    Some((name, rest)) => (name, rest.trim()),
                                    None => (cmd, ""),
                                };

                                if let Some(skill) = app_state.skills.get(skill_name).cloned() {
                                    let full_input = if args.is_empty() {
                                        String::new()
                                    } else {
                                        args.to_string()
                                    };
                                    handle_skill_input(
                                        &mut app_state, &mut ui_state, &ollama_client, &skill, &full_input,
                                    );
                                } else {
                                    ui_state.add_output(OutputLine::Error(
                                        format!("Unknown skill: /{}. Type /skills to see available skills.", skill_name),
                                    ));
                                }
                            } else {
                                // Process the input through agent pipeline
                                handle_input(&mut app_state, &mut ui_state, &ollama_client, &input);
                            }
                        }
                    }
                    // Backspace
                    (KeyModifiers::NONE, KeyCode::Backspace) => {
                        ui_state.backspace();
                    }
                    // Esc: toggle sidebar
                    (KeyModifiers::NONE, KeyCode::Esc) => {
                        ui_state.sidebar_visible = !ui_state.sidebar_visible;
                    }
                    // Character input
                    (KeyModifiers::NONE, KeyCode::Char(c)) => {
                        ui_state.push_char(c);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn handle_input(
    app_state: &mut AppState,
    ui_state: &mut ui::UiState,
    client: &ollama::OllamaClient,
    input: &str,
) {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            ui_state.add_output(OutputLine::Error(format!("Runtime error: {}", e)));
            return;
        }
    };

    let model = &app_state.config.core_model;
    if model.is_empty() {
        ui_state.add_output(OutputLine::Error(
            "No core model configured. Run setup or edit ~/.litecode/config.toml".into(),
        ));
        return;
    }

    // Detect if this looks like a code implementation request
    let kind = if looks_like_code_request(input) {
        agent::retry::ResponseKind::CodeImplementation
    } else {
        agent::retry::ResponseKind::Chat
    };

    let max_retries = app_state.config.max_retries;

    let result = rt.block_on(agent::retry::chat_with_retry(
        client,
        model,
        agent::prompts::CODING_SYSTEM,
        input,
        kind,
        max_retries,
    ));

    match result {
        agent::retry::RetryResult::Success { content, attempts } => {
            if attempts > 0 {
                ui_state.add_output(OutputLine::System(
                    format!("Got valid response after {} retries", attempts),
                ));
            }
            ui_state.add_output(OutputLine::Assistant(content));
        }
        agent::retry::RetryResult::Exhausted { content, attempts, corrections } => {
            ui_state.add_output(OutputLine::Error(
                format!("Response still invalid after {} retries. Showing last attempt:", attempts),
            ));
            for (_, reason) in &corrections {
                ui_state.add_output(OutputLine::Error(format!("  - {}", reason)));
            }
            ui_state.add_output(OutputLine::Assistant(content));
        }
        agent::retry::RetryResult::Failed { last_error, .. } => {
            ui_state.add_output(OutputLine::Error(last_error));
        }
    }
}

fn handle_skill_input(
    app_state: &mut AppState,
    ui_state: &mut ui::UiState,
    client: &ollama::OllamaClient,
    skill: &skills::Skill,
    args: &str,
) {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            ui_state.add_output(OutputLine::Error(format!("Runtime error: {}", e)));
            return;
        }
    };

    let model = &app_state.config.core_model;
    if model.is_empty() {
        ui_state.add_output(OutputLine::Error(
            "No core model configured. Run setup or edit ~/.litecode/config.toml".into(),
        ));
        return;
    }

    let user_message = if args.is_empty() {
        ui_state.add_output(OutputLine::Error(
            format!("Usage: /{} <question or description>", skill.name),
        ));
        return;
    } else {
        args.to_string()
    };

    let system_prompt = format!("{}\n\n{}", agent::prompts::CODING_SYSTEM, skill.content);
    let max_retries = app_state.config.max_retries;

    // Skills generally produce chat-style output; code skills get code validation
    let kind = if skill.name == "simplify" || skill.name == "review" || skill.name == "test" {
        agent::retry::ResponseKind::CodeImplementation
    } else {
        agent::retry::ResponseKind::Chat
    };

    let result = rt.block_on(agent::retry::chat_with_retry(
        client,
        model,
        &system_prompt,
        &user_message,
        kind,
        max_retries,
    ));

    match result {
        agent::retry::RetryResult::Success { content, attempts } => {
            if attempts > 0 {
                ui_state.add_output(OutputLine::System(
                    format!("Got valid response after {} retries", attempts),
                ));
            }
            ui_state.add_output(OutputLine::Assistant(content));
        }
        agent::retry::RetryResult::Exhausted { content, attempts, corrections } => {
            ui_state.add_output(OutputLine::Error(
                format!("Response still invalid after {} retries. Showing last attempt:", attempts),
            ));
            for (_, reason) in &corrections {
                ui_state.add_output(OutputLine::Error(format!("  - {}", reason)));
            }
            ui_state.add_output(OutputLine::Assistant(content));
        }
        agent::retry::RetryResult::Failed { last_error, .. } => {
            ui_state.add_output(OutputLine::Error(last_error));
        }
    }
}

/// Heuristic: does this user input look like it's asking for code generation?
fn looks_like_code_request(input: &str) -> bool {
    let lower = input.to_lowercase();
    let code_keywords = [
        "implement", "create", "write", "build", "add", "refactor",
        "generate", "fix", "modify", "change", "update", "make a",
        "code", "function", "class", "module", "file",
    ];
    code_keywords.iter().any(|k| lower.contains(k))
}
