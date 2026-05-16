mod app;
mod codebase;
mod config;
mod ollama;
mod project;
mod sandbox;
mod search;
mod session;
mod ui;
mod util;
mod agent;

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
        "Shift+Tab: switch mode | Enter: send | Ctrl+C: quit".into(),
    ));

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

                            // Process the input through agent pipeline
                            handle_input(&mut app_state, &mut ui_state, &ollama_client, &input);
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
    // Use a simple synchronous approach for now (streaming will use tokio spawn in future)
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            ui_state.add_output(OutputLine::Error(format!("Runtime error: {}", e)));
            return;
        }
    };

    let messages = vec![
        ollama::chat::ChatMessage::system(agent::prompts::CODING_SYSTEM),
        ollama::chat::ChatMessage::user(input),
    ];

    let model = &app_state.config.core_model;
    if model.is_empty() {
        ui_state.add_output(OutputLine::Error(
            "No core model configured. Run setup or edit ~/.litecode/config.toml".into(),
        ));
        return;
    }

    match rt.block_on(client.chat(model, messages)) {
        Ok(response) => {
            ui_state.add_output(OutputLine::Assistant(response.content));
        }
        Err(e) => {
            ui_state.add_output(OutputLine::Error(format!("Ollama error: {}", e)));
        }
    }
}
