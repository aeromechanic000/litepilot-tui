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
use std::sync::mpsc;
use std::time::Duration;
use ui::OutputLine;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "litepilot",
    version,
    about = "Terminal AI coding assistant powered by local Ollama models"
)]
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

    // Setup config — use project-local .litepilot if present, else global ~/.litepilot
    let litepilot_dir = config::Config::ensure_dirs_for(&workspace)?;
    let config = config::Config::load_for_workspace(&workspace).unwrap_or_else(|_| {
        let default = config::Config::default();
        let _ = default.save(&litepilot_dir.join("config.toml"));
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

    // Run app with panic recovery to ensure terminal is restored
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_app(&mut terminal, config, workspace)
    }));

    // Restore terminal (even if panic occurred)
    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    match result {
        Ok(inner) => inner,
        Err(panic_payload) => {
            if let Some(msg) = panic_payload.downcast_ref::<&str>() {
                eprintln!("Fatal error: {}", msg);
            } else if let Some(msg) = panic_payload.downcast_ref::<String>() {
                eprintln!("Fatal error: {}", msg);
            } else {
                eprintln!("Fatal error: unknown panic");
            }
            std::process::exit(1);
        }
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: config::Config,
    workspace: PathBuf,
) -> Result<()> {
    let mut ui_state = ui::UiState::from_config(&config).with_workspace(workspace.clone());
    let mut app_state = AppState::new(config, workspace);

    // Welcome message
    ui_state.add_output(OutputLine::System(
        "Welcome to LitePilot! Ollama-powered local coding agent.".into(),
    ));
    ui_state.add_output(OutputLine::System(
        "Shift+Tab: switch mode | Enter: send | Ctrl+C: quit | /skills: list skills".into(),
    ));

    // Show loaded skills count
    let skill_count = app_state.skills.list().len();
    if skill_count > 0 {
        let names: Vec<&str> = app_state
            .skills
            .list()
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        ui_state.add_output(OutputLine::System(format!(
            "Loaded {} skills: /{}",
            skill_count,
            names.join(", /")
        )));
    }

    let ollama_client = ollama::OllamaClient::new(&app_state.config)?;

    // Populate sidebar file tree
    {
        let ctx = project::ProjectContext::new(app_state.workspace.clone());
        let entries = ctx.list_tree();
        let ui_entries: Vec<ui::FileEntry> = entries
            .into_iter()
            .map(|e| ui::FileEntry {
                path: e.path.to_string_lossy().to_string(),
                is_dir: e.is_dir,
                depth: e.depth,
                expanded: false,
            })
            .collect();
        ui_state.set_file_tree(ui_entries);
    }

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
        ui_state.add_output(OutputLine::System(format!(
            "Connected to Ollama at {}",
            app_state.config.ollama_endpoint
        )));
    } else {
        ui_state.add_output(OutputLine::Error(format!(
            "Cannot connect to Ollama at {}. Start Ollama first.",
            app_state.config.ollama_endpoint
        )));
    }

    // Channel for receiving LLM results from background threads
    let (result_tx, result_rx) = mpsc::channel::<agent::retry::PipelineResult>();

    loop {
        terminal.draw(|f| ui::draw(f, &app_state, &mut ui_state))?;

        // Check for completed LLM responses (non-blocking)
        while let Ok(result) = result_rx.try_recv() {
            match result {
                agent::retry::PipelineResult::Retry(r) => {
                    app_state.is_processing = false;
                    render_retry_result(&mut ui_state, r);
                }
                agent::retry::PipelineResult::AutoSuccess { changes, applied } => {
                    app_state.is_processing = false;
                    render_auto_result(&mut ui_state, &changes, &applied);
                }
                agent::retry::PipelineResult::AutoFailed { error } => {
                    app_state.is_processing = false;
                    ui_state.add_output(OutputLine::Error(format!("Pipeline failed: {}", error)));
                }
                agent::retry::PipelineResult::SearchDone { count, .. } => {
                    // Intermediate status — LLM still running
                    ui_state.add_output(OutputLine::System(format!(
                        "[search] Found {} result(s)",
                        count
                    )));
                    continue; // Don't drain queue yet
                }
                agent::retry::PipelineResult::StreamChunk { content } => {
                    // Append chunk to streaming output — don't mark processing done
                    ui_state.append_stream_chunk(&content);
                    continue;
                }
                agent::retry::PipelineResult::StreamDone { content } => {
                    app_state.is_processing = false;
                    ui_state.finish_stream();
                    // Check for file changes and show /apply hint
                    if !content.is_empty() {
                        let changes = agent::AgentPipeline::parse_file_changes(&content);
                        if !changes.is_empty() {
                            ui_state.add_output(OutputLine::System(format!(
                                "Detected {} file change(s). Type /apply to write.",
                                changes.len()
                            )));
                        }
                    }
                }
            }

            // Drain next queued message if any
            if !app_state.pending_queue.is_empty() {
                let next = app_state.pending_queue.remove(0);
                ui_state.add_output(OutputLine::System(
                    "Processing queued message...".to_string(),
                ));
                ui_state.add_output(OutputLine::User(next.clone()));
                spawn_request_for_mode(&app_state, &ollama_client, &next, result_tx.clone());
                app_state.is_processing = true;
            }
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Handle edit confirmation keys (y/n/a) when awaiting
                if app_state.awaiting_confirmation && key.modifiers == KeyModifiers::NONE {
                    match key.code {
                        KeyCode::Char('y') => {
                            // Apply the next pending file
                            if let Some(action) = app_state.pending_confirmations.pop() {
                                let sandbox = sandbox::Sandbox::new(app_state.workspace.clone());
                                apply_pending_action(&mut ui_state, &app_state, action, &sandbox);
                            }
                            if app_state.pending_confirmations.is_empty() {
                                app_state.awaiting_confirmation = false;
                                ui_state
                                    .add_output(OutputLine::System("All files reviewed.".into()));
                            } else {
                                ui_state.add_output(OutputLine::System(format!(
                                    "Apply next? ({} remaining) y/n/a:",
                                    app_state.pending_confirmations.len()
                                )));
                            }
                            continue;
                        }
                        KeyCode::Char('n') => {
                            // Skip this file
                            if let Some(action) = app_state.pending_confirmations.pop() {
                                let path = match &action {
                                    app::PendingAction::WriteFile { path, .. } => {
                                        path.display().to_string()
                                    }
                                    app::PendingAction::DeleteFile { path } => {
                                        path.display().to_string()
                                    }
                                    app::PendingAction::ExecuteCommand { cmd, .. } => cmd.clone(),
                                };
                                ui_state
                                    .add_output(OutputLine::System(format!("Skipped {}", path)));
                            }
                            if app_state.pending_confirmations.is_empty() {
                                app_state.awaiting_confirmation = false;
                                ui_state
                                    .add_output(OutputLine::System("All files reviewed.".into()));
                            } else {
                                ui_state.add_output(OutputLine::System(format!(
                                    "Apply next? ({} remaining) y/n/a:",
                                    app_state.pending_confirmations.len()
                                )));
                            }
                            continue;
                        }
                        KeyCode::Char('a') => {
                            // Apply all remaining
                            let sandbox = sandbox::Sandbox::new(app_state.workspace.clone());
                            let remaining = std::mem::take(&mut app_state.pending_confirmations);
                            let total = remaining.len();
                            let mut applied = 0;
                            for action in remaining {
                                apply_pending_action(&mut ui_state, &app_state, action, &sandbox);
                                applied += 1;
                            }
                            app_state.awaiting_confirmation = false;
                            ui_state.add_output(OutputLine::System(format!(
                                "Applied {}/{} remaining file(s)",
                                applied, total
                            )));
                            continue;
                        }
                        _ => {} // Fall through to normal key handling
                    }
                }

                match (key.modifiers, key.code) {
                    // Shift+Tab: switch mode
                    (KeyModifiers::SHIFT, KeyCode::Tab) => {
                        let new_mode = app_state.switch_mode();
                        ui_state.add_output(OutputLine::System(format!(
                            "Switched to {} mode",
                            new_mode
                        )));
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
                                    if k2.code == KeyCode::Char('c')
                                        && k2.modifiers.contains(KeyModifiers::CONTROL)
                                    {
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
                        let state = if app_state.web_search_enabled {
                            "ON"
                        } else {
                            "OFF"
                        };
                        ui_state.add_output(OutputLine::System(format!("Web search: {}", state)));
                    }
                    // Enter: submit input
                    (KeyModifiers::NONE, KeyCode::Enter) => {
                        let input = ui_state.take_input();
                        if !input.is_empty() {
                            app_state.input_history.push(input.clone());

                            if input.trim() == "/quit" || input.trim() == "/exit" {
                                break;
                            }

                            // Handle slash commands (instant, no LLM call)
                            let trimmed = input.trim();
                            if trimmed == "/skills" {
                                let skills = app_state.skills.list();
                                if skills.is_empty() {
                                    ui_state
                                        .add_output(OutputLine::System("No skills loaded.".into()));
                                } else {
                                    let mut lines = vec!["Available skills:".to_string()];
                                    for s in skills {
                                        lines.push(format!("  /{} — {}", s.name, s.description));
                                    }
                                    ui_state.add_output(OutputLine::System(lines.join("\n")));
                                }
                            } else if trimmed == "/setup" {
                                match wizard::run(
                                    terminal,
                                    app_state.config.clone(),
                                    &app_state.workspace,
                                ) {
                                    Ok(new_config) => {
                                        app_state.config = new_config;
                                        ui_state.add_output(OutputLine::System(
                                            "Setup complete. Configuration updated.".into(),
                                        ));
                                    }
                                    Err(e) => {
                                        ui_state.add_output(OutputLine::Error(format!(
                                            "Setup wizard failed: {}",
                                            e
                                        )));
                                    }
                                }
                                let skills = app_state.skills.list();
                                if skills.is_empty() {
                                    ui_state
                                        .add_output(OutputLine::System("No skills loaded.".into()));
                                } else {
                                    let mut lines = vec!["Available skills:".to_string()];
                                    for s in skills {
                                        lines.push(format!("  /{} — {}", s.name, s.description));
                                    }
                                    ui_state.add_output(OutputLine::System(lines.join("\n")));
                                }
                            } else if trimmed == "/apply" {
                                // Apply file changes from the last assistant response
                                let last_content =
                                    ui_state.output_lines.iter().rev().find_map(|ol| match ol {
                                        OutputLine::Assistant(t) => Some(t.clone()),
                                        _ => None,
                                    });
                                if let Some(ref content) = last_content {
                                    let changes = agent::AgentPipeline::parse_file_changes(content);
                                    if changes.is_empty() {
                                        ui_state.add_output(OutputLine::Error(
                                            "No file changes found in the last response.".into(),
                                        ));
                                    } else if app_state.mode == AppMode::Edit {
                                        // Edit mode: queue confirmations for y/n review
                                        let sandbox =
                                            sandbox::Sandbox::new(app_state.workspace.clone());
                                        let file_ops = project::file_ops::FileOps::new(
                                            &sandbox,
                                            app_state.mode,
                                        );
                                        app_state.clear_pending();
                                        let mut queued = 0;
                                        for change in &changes {
                                            let full_path = app_state.workspace.join(&change.path);
                                            let fc = match if change.action == "delete" {
                                                file_ops.prepare_delete(&full_path)
                                            } else {
                                                file_ops.prepare_write(&full_path, &change.content)
                                            } {
                                                Ok(fc) => fc,
                                                Err(e) => {
                                                    ui_state.add_output(OutputLine::Error(
                                                        format!(
                                                            "Blocked {}: {}",
                                                            change.path.display(),
                                                            e
                                                        ),
                                                    ));
                                                    continue;
                                                }
                                            };

                                            // Show diff preview
                                            if !fc.diff_preview.is_empty() {
                                                ui_state.add_output(OutputLine::System(format!(
                                                    "--- {}",
                                                    change.path.display()
                                                )));
                                                for line in fc.diff_preview.lines() {
                                                    if line.starts_with('-')
                                                        && !line.starts_with("---")
                                                    {
                                                        ui_state.add_output(OutputLine::Diff {
                                                            added: vec![],
                                                            removed: vec![line.to_string()],
                                                        });
                                                    } else if line.starts_with('+')
                                                        && !line.starts_with("+++")
                                                    {
                                                        ui_state.add_output(OutputLine::Diff {
                                                            added: vec![line.to_string()],
                                                            removed: vec![],
                                                        });
                                                    }
                                                }
                                            }

                                            let action = if change.action == "delete" {
                                                app::PendingAction::DeleteFile { path: full_path }
                                            } else {
                                                app::PendingAction::WriteFile {
                                                    path: full_path,
                                                    content: change.content.clone(),
                                                    diff_preview: fc.diff_preview,
                                                }
                                            };
                                            app_state.push_pending(action);
                                            queued += 1;
                                        }
                                        if queued > 0 {
                                            app_state.awaiting_confirmation = true;
                                            ui_state.add_output(OutputLine::System(format!(
                                                "Review {} file(s). Press y/n/a (y=yes, n=no, a=apply all):",
                                                queued
                                            )));
                                        }
                                    } else {
                                        // Auto mode: apply all immediately (Plan mode blocked by FileOps)
                                        let sandbox =
                                            sandbox::Sandbox::new(app_state.workspace.clone());
                                        let file_ops = project::file_ops::FileOps::new(
                                            &sandbox,
                                            app_state.mode,
                                        );
                                        let mut applied = 0;
                                        for change in &changes {
                                            let full_path = app_state.workspace.join(&change.path);
                                            let fc = match if change.action == "delete" {
                                                file_ops.prepare_delete(&full_path)
                                            } else {
                                                file_ops.prepare_write(&full_path, &change.content)
                                            } {
                                                Ok(fc) => fc,
                                                Err(e) => {
                                                    ui_state.add_output(OutputLine::Error(
                                                        format!(
                                                            "Blocked {}: {}",
                                                            change.path.display(),
                                                            e
                                                        ),
                                                    ));
                                                    continue;
                                                }
                                            };
                                            match file_ops.apply_change(&fc) {
                                                Ok(()) => {
                                                    ui_state.add_output(OutputLine::System(
                                                        format!("Wrote {}", change.path.display()),
                                                    ));
                                                    applied += 1;
                                                    run_syntax_check(
                                                        &mut ui_state,
                                                        &app_state.workspace.join(&change.path),
                                                        &sandbox,
                                                    );
                                                }
                                                Err(e) => {
                                                    ui_state.add_output(OutputLine::Error(
                                                        format!(
                                                            "Failed to write {}: {}",
                                                            change.path.display(),
                                                            e
                                                        ),
                                                    ));
                                                }
                                            }
                                        }
                                        ui_state.add_output(OutputLine::System(format!(
                                            "Applied {}/{} file(s)",
                                            applied,
                                            changes.len()
                                        )));
                                    }
                                } else {
                                    ui_state.add_output(OutputLine::Error(
                                        "No assistant response to apply.".into(),
                                    ));
                                }
                            } else if let Some(uv_args) = trimmed.strip_prefix("/uv ") {
                                // /uv commands: init, venv, add <pkg>, run <script>
                                handle_uv_command(&mut app_state, &mut ui_state, uv_args.trim());
                            } else if trimmed == "/uv" {
                                ui_state.add_output(OutputLine::System(
                                    "Usage: /uv init | /uv venv | /uv add <package> | /uv run <script>".into(),
                                ));
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
                                    if app_state.is_processing {
                                        // Queue skill invocation
                                        let label = format!("/{} {}", skill_name, full_input)
                                            .trim_end()
                                            .to_string();
                                        app_state.pending_queue.push(label.clone());
                                        ui_state.add_output(OutputLine::Pending(label));
                                    } else {
                                        ui_state.add_output(OutputLine::User(input.clone()));
                                        spawn_skill_request(
                                            &app_state,
                                            &ollama_client,
                                            &skill,
                                            &full_input,
                                            result_tx.clone(),
                                        );
                                        app_state.is_processing = true;
                                    }
                                } else {
                                    ui_state.add_output(OutputLine::Error(format!(
                                        "Unknown skill: /{}. Type /skills to see available skills.",
                                        skill_name
                                    )));
                                }
                            } else if app_state.is_processing {
                                // Queue the message for later processing
                                app_state.pending_queue.push(input.clone());
                                ui_state.add_output(OutputLine::Pending(input));
                            } else {
                                // Display user message immediately and spawn background request
                                ui_state.add_output(OutputLine::User(input.clone()));
                                spawn_request_for_mode(
                                    &app_state,
                                    &ollama_client,
                                    &input,
                                    result_tx.clone(),
                                );
                                app_state.is_processing = true;
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
                    // Page Up: scroll chat up
                    (KeyModifiers::NONE, KeyCode::PageUp) => {
                        ui_state.scroll_up(10);
                    }
                    // Page Down: scroll chat down
                    (KeyModifiers::NONE, KeyCode::PageDown) => {
                        ui_state.scroll_down(10);
                    }
                    // Tab: switch sidebar tab (only when sidebar visible)
                    (KeyModifiers::NONE, KeyCode::Tab) => {
                        if ui_state.sidebar_visible {
                            ui_state.sidebar_switch_tab();
                        }
                    }
                    // Up arrow in sidebar
                    (KeyModifiers::NONE, KeyCode::Up) => {
                        if ui_state.sidebar_visible {
                            ui_state.sidebar_move_up();
                        }
                    }
                    // Down arrow in sidebar
                    (KeyModifiers::NONE, KeyCode::Down) => {
                        if ui_state.sidebar_visible {
                            ui_state.sidebar_move_down();
                        }
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

/// Spawn a streaming chat request on a background thread.
fn spawn_llm_request(
    app_state: &AppState,
    _client: &ollama::OllamaClient,
    input: &str,
    tx: mpsc::Sender<agent::retry::PipelineResult>,
) {
    let model = app_state.config.core_model.clone();
    if model.is_empty() {
        let _ = tx.send(agent::retry::PipelineResult::Retry(
            agent::retry::RetryResult::Failed {
                last_error: "No core model configured. Run setup or edit ~/.litepilot/config.toml"
                    .into(),
                attempts: 0,
            },
        ));
        return;
    }

    let endpoint = app_state.config.ollama_endpoint.clone();
    let connect_timeout = app_state.config.connect_timeout;
    let input = input.to_string();
    let web_search_enabled = app_state.web_search_enabled;
    let max_search_tokens = app_state.config.max_search_context_tokens;

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                let _ = tx.send(agent::retry::PipelineResult::Retry(
                    agent::retry::RetryResult::Failed {
                        last_error: format!("Runtime error: {}", e),
                        attempts: 0,
                    },
                ));
                return;
            }
        };

        // Run web search if enabled
        let user_message = if web_search_enabled {
            let search_ctx = rt.block_on(run_web_search(&input, max_search_tokens));
            if !search_ctx.is_empty() {
                let _ = tx.send(agent::retry::PipelineResult::SearchDone {
                    count: search_ctx.lines().filter(|l| l.starts_with('[')).count(),
                    context: search_ctx.clone(),
                });
            }
            format!("{}\n{}", search_ctx, input)
        } else {
            input
        };

        // Build streaming request
        let bg_config = config::Config {
            ollama_endpoint: endpoint.clone(),
            connect_timeout,
            ..config::Config::default()
        };
        let bg_client = match ollama::OllamaClient::new(&bg_config) {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(agent::retry::PipelineResult::Retry(
                    agent::retry::RetryResult::Failed {
                        last_error: format!("Client error: {}", e),
                        attempts: 0,
                    },
                ));
                return;
            }
        };

        let http = bg_client.http_client();
        let messages = vec![
            ollama::chat::ChatMessage::system(agent::prompts::CODING_SYSTEM),
            ollama::chat::ChatMessage::user(&user_message),
        ];
        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

        let stream = ollama::OllamaClient::chat_stream(http, endpoint, model, messages, cancel_rx);

        let mut pin = std::pin::pin!(stream);

        let result_content: Result<String, String> = rt.block_on(async {
            use futures::StreamExt;
            let mut full_content = String::new();
            while let Some(chunk_result) = pin.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        if chunk.done {
                            break;
                        }
                        if !chunk.content.is_empty() {
                            let _ = tx.send(agent::retry::PipelineResult::StreamChunk {
                                content: chunk.content.clone(),
                            });
                            full_content.push_str(&chunk.content);
                        }
                    }
                    Err(e) => {
                        return Err(format!("Stream error: {}", e));
                    }
                }
            }
            Ok(full_content)
        });

        // Drop cancel sender to clean up
        drop(cancel_tx);

        match result_content {
            Ok(content) => {
                let _ = tx.send(agent::retry::PipelineResult::StreamDone { content });
            }
            Err(error) => {
                let _ = tx.send(agent::retry::PipelineResult::Retry(
                    agent::retry::RetryResult::Failed {
                        last_error: error,
                        attempts: 0,
                    },
                ));
            }
        }
    });
}

/// Spawn a skill-based request on a background thread.
fn spawn_skill_request(
    app_state: &AppState,
    _client: &ollama::OllamaClient,
    skill: &skills::Skill,
    args: &str,
    tx: mpsc::Sender<agent::retry::PipelineResult>,
) {
    let model = app_state.config.core_model.clone();
    if model.is_empty() {
        let _ = tx.send(agent::retry::PipelineResult::Retry(
            agent::retry::RetryResult::Failed {
                last_error: "No core model configured. Run setup or edit ~/.litepilot/config.toml"
                    .into(),
                attempts: 0,
            },
        ));
        return;
    }

    if args.is_empty() {
        let _ = tx.send(agent::retry::PipelineResult::Retry(
            agent::retry::RetryResult::Failed {
                last_error: format!("Usage: /{} <question or description>", skill.name),
                attempts: 0,
            },
        ));
        return;
    }

    let system_prompt = format!("{}\n\n{}", agent::prompts::CODING_SYSTEM, skill.content);
    let max_retries = app_state.config.max_retries;
    let endpoint = app_state.config.ollama_endpoint.clone();
    let connect_timeout = app_state.config.connect_timeout;
    let user_message = args.to_string();
    let kind = if skill.name == "simplify" || skill.name == "review" || skill.name == "test" {
        agent::retry::ResponseKind::CodeImplementation
    } else {
        agent::retry::ResponseKind::Chat
    };

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                let _ = tx.send(agent::retry::PipelineResult::Retry(
                    agent::retry::RetryResult::Failed {
                        last_error: format!("Runtime error: {}", e),
                        attempts: 0,
                    },
                ));
                return;
            }
        };

        let bg_config = config::Config {
            ollama_endpoint: endpoint,
            connect_timeout,
            ..config::Config::default()
        };
        let bg_client = match ollama::OllamaClient::new(&bg_config) {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(agent::retry::PipelineResult::Retry(
                    agent::retry::RetryResult::Failed {
                        last_error: format!("Client error: {}", e),
                        attempts: 0,
                    },
                ));
                return;
            }
        };

        let result = rt.block_on(agent::retry::chat_with_retry(
            &bg_client,
            &model,
            &system_prompt,
            &user_message,
            kind,
            max_retries,
        ));
        let _ = tx.send(agent::retry::PipelineResult::Retry(result));
    });
}

fn render_retry_result(ui_state: &mut ui::UiState, result: agent::retry::RetryResult) {
    let content = match &result {
        agent::retry::RetryResult::Success { content, attempts } => {
            if *attempts > 0 {
                ui_state.add_output(OutputLine::System(format!(
                    "Got valid response after {} retries",
                    attempts
                )));
            }
            content.clone()
        }
        agent::retry::RetryResult::Exhausted {
            content,
            attempts,
            corrections,
        } => {
            ui_state.add_output(OutputLine::Error(format!(
                "Response still invalid after {} retries. Showing last attempt:",
                attempts
            )));
            for (_, reason) in corrections {
                ui_state.add_output(OutputLine::Error(format!("  - {}", reason)));
            }
            content.clone()
        }
        agent::retry::RetryResult::Failed { last_error, .. } => {
            ui_state.add_output(OutputLine::Error(last_error.clone()));
            return;
        }
    };

    // Display the assistant response
    ui_state.add_output(OutputLine::Assistant(content.clone()));

    // If the response contains file changes, present them for review
    let changes = agent::AgentPipeline::parse_file_changes(&content);
    if !changes.is_empty() {
        ui_state.add_output(OutputLine::System(format!(
            "Detected {} file change(s):",
            changes.len()
        )));
        for change in &changes {
            let action_icon = match change.action.as_str() {
                "create" => "+",
                "modify" => "~",
                "delete" => "-",
                _ => "?",
            };
            ui_state.add_output(OutputLine::System(format!(
                "  {} {} ({})",
                action_icon,
                change.path.display(),
                change.action
            )));

            // Show diff preview for modifications
            if change.action == "modify" {
                let full_path = ui_state.workspace_hint.join(&change.path);
                if full_path.exists() {
                    if let Ok(old_content) = std::fs::read_to_string(&full_path) {
                        let diff = util::diff::generate_unified_diff(
                            &old_content,
                            &change.content,
                            &change.path.to_string_lossy(),
                        );
                        let removed: Vec<String> = diff
                            .lines()
                            .filter(|l| l.starts_with('-') && !l.starts_with("---"))
                            .map(|l| l.to_string())
                            .collect();
                        let added: Vec<String> = diff
                            .lines()
                            .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
                            .map(|l| l.to_string())
                            .collect();
                        ui_state.add_output(OutputLine::Diff { added, removed });
                    }
                }
            }
        }
        ui_state.add_output(OutputLine::System(
            "Type /apply to write these files, or ignore to discard.".into(),
        ));
    }
}

/// Route request based on current mode: Auto+code → pipeline, else → direct chat.
fn spawn_request_for_mode(
    app_state: &AppState,
    client: &ollama::OllamaClient,
    input: &str,
    tx: mpsc::Sender<agent::retry::PipelineResult>,
) {
    if app_state.mode == app::AppMode::Auto && looks_like_code_request(input) {
        spawn_auto_pipeline(app_state, input, tx);
    } else {
        spawn_llm_request(app_state, client, input, tx);
    }
}

/// Spawn the full plan→implement→audit pipeline on a background thread.
fn spawn_auto_pipeline(
    app_state: &AppState,
    input: &str,
    tx: mpsc::Sender<agent::retry::PipelineResult>,
) {
    let workspace = app_state.workspace.clone();
    let endpoint = app_state.config.ollama_endpoint.clone();
    let connect_timeout = app_state.config.connect_timeout;
    let max_retries = app_state.config.max_retries;
    let input = input.to_string();
    let web_search_enabled = app_state.web_search_enabled;
    let max_search_tokens = app_state.config.max_search_context_tokens;

    // Clone config values needed by the background thread
    let fast_model = app_state.config.effective_fast_model().to_string();
    let core_model = app_state.config.core_model.clone();
    let audit_model = app_state.config.effective_audit_model().to_string();

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                let _ = tx.send(agent::retry::PipelineResult::AutoFailed {
                    error: format!("Runtime error: {}", e),
                });
                return;
            }
        };

        // Run web search if enabled
        let user_message = if web_search_enabled {
            let search_ctx = rt.block_on(run_web_search(&input, max_search_tokens));
            if !search_ctx.is_empty() {
                let _ = tx.send(agent::retry::PipelineResult::SearchDone {
                    count: search_ctx.lines().filter(|l| l.starts_with('[')).count(),
                    context: search_ctx.clone(),
                });
            }
            format!("{}\n{}", search_ctx, input)
        } else {
            input
        };

        let bg_config = config::Config {
            ollama_endpoint: endpoint,
            connect_timeout,
            fast_model,
            core_model,
            audit_model,
            max_retries,
            ..config::Config::default()
        };
        let bg_client = match ollama::OllamaClient::new(&bg_config) {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(agent::retry::PipelineResult::AutoFailed {
                    error: format!("Client error: {}", e),
                });
                return;
            }
        };
        let sandbox = sandbox::Sandbox::new(workspace.clone());

        let result = rt.block_on(agent::auto_run::run_auto_pipeline(
            &bg_client,
            &bg_config,
            &sandbox,
            workspace,
            &user_message,
            "",   // no project context for now
            None, // no codebase for now
        ));

        match result {
            Ok(changes) => {
                let applied: Vec<String> = changes
                    .iter()
                    .map(|c| format!("{} ({})", c.path.display(), c.action))
                    .collect();
                let _ = tx.send(agent::retry::PipelineResult::AutoSuccess { changes, applied });
            }
            Err(e) => {
                let _ = tx.send(agent::retry::PipelineResult::AutoFailed {
                    error: e.to_string(),
                });
            }
        }
    });
}

/// Render auto pipeline results in the chat panel.
fn render_auto_result(
    ui_state: &mut ui::UiState,
    changes: &[agent::FileChange],
    applied: &[String],
) {
    if changes.is_empty() {
        ui_state.add_output(OutputLine::System(
            "Pipeline completed but produced no changes.".into(),
        ));
        return;
    }

    ui_state.add_output(OutputLine::System(format!(
        "Auto pipeline: {} file(s) generated and applied.",
        applied.len()
    )));
    for path in applied {
        ui_state.add_output(OutputLine::System(format!("  + {}", path)));
    }
    for change in changes {
        // Show content summary
        let line_count = change.content.lines().count();
        ui_state.add_output(OutputLine::Assistant(format!(
            "### FILE: {}\n### ACTION: {}\n```{}\n```\n",
            change.path.display(),
            change.action,
            if line_count > 20 {
                format!(
                    "{} ({} lines)",
                    &change
                        .content
                        .lines()
                        .take(20)
                        .collect::<Vec<_>>()
                        .join("\n"),
                    line_count
                )
            } else {
                change.content.clone()
            }
        )));
    }
}

/// Write a FileChange to disk, with sandbox validation.
/// Run web search and return formatted context string.
/// Returns empty string if search fails or is disabled.
async fn run_web_search(query: &str, max_tokens: usize) -> String {
    let config = config::Config::default();
    let engine = search::SearchEngine::new(&config);
    match engine.search(query, max_tokens).await {
        Ok(results) if !results.is_empty() => search::format_search_context(&results),
        _ => String::new(),
    }
}

/// Handle /uv subcommands: init, venv, add <pkg>, run <script>.
fn handle_uv_command(app_state: &mut AppState, ui_state: &mut ui::UiState, args: &str) {
    if !project::uv::UvManager::is_available() {
        ui_state.add_output(OutputLine::Error(
            "uv is not installed. Install it: https://docs.astral.sh/uv/".into(),
        ));
        return;
    }

    if !app_state.mode.can_execute_command() {
        ui_state.add_output(OutputLine::Error(
            "Command execution not allowed in Plan mode.".into(),
        ));
        return;
    }

    let (subcmd, rest) = match args.split_once(' ') {
        Some((s, r)) => (s, r.trim()),
        None => (args, ""),
    };

    let sandbox = sandbox::Sandbox::new(app_state.workspace.clone());
    let workspace = app_state.workspace.clone();

    let result = match subcmd {
        "init" => {
            ui_state.add_output(OutputLine::System("Running uv init...".into()));
            std::thread::scope(|s| {
                s.spawn(|| {
                    let rt = tokio::runtime::Runtime::new().ok()?;
                    rt.block_on(project::uv::UvManager::init(&workspace, &sandbox))
                        .ok()
                })
                .join()
                .ok()?
            })
        }
        "venv" => {
            ui_state.add_output(OutputLine::System("Running uv venv...".into()));
            std::thread::scope(|s| {
                s.spawn(|| {
                    let rt = tokio::runtime::Runtime::new().ok()?;
                    rt.block_on(project::uv::UvManager::create_venv(&workspace, &sandbox))
                        .ok()
                })
                .join()
                .ok()?
            })
        }
        "add" => {
            if rest.is_empty() {
                ui_state.add_output(OutputLine::Error("Usage: /uv add <package>".into()));
                return;
            }
            ui_state.add_output(OutputLine::System(format!("Running uv add {}...", rest)));
            std::thread::scope(|s| {
                s.spawn(|| {
                    let rt = tokio::runtime::Runtime::new().ok()?;
                    rt.block_on(project::uv::UvManager::add(&workspace, rest, &sandbox))
                        .ok()
                })
                .join()
                .ok()?
            })
        }
        "run" => {
            if rest.is_empty() {
                ui_state.add_output(OutputLine::Error("Usage: /uv run <script>".into()));
                return;
            }
            ui_state.add_output(OutputLine::System(format!("Running uv run {}...", rest)));
            std::thread::scope(|s| {
                s.spawn(|| {
                    let rt = tokio::runtime::Runtime::new().ok()?;
                    rt.block_on(project::uv::UvManager::run(&workspace, rest, &sandbox))
                        .ok()
                })
                .join()
                .ok()?
            })
        }
        _ => {
            ui_state.add_output(OutputLine::Error(format!(
                "Unknown /uv subcommand: {}. Use: init, venv, add, run",
                subcmd
            )));
            return;
        }
    };

    match result {
        Some(output) => {
            if output.success {
                if !output.stdout.is_empty() {
                    ui_state.add_output(OutputLine::System(output.stdout));
                }
                ui_state.add_output(OutputLine::System("Done.".into()));
            } else {
                ui_state.add_output(OutputLine::Error(format!(
                    "uv {} failed: {}",
                    subcmd,
                    if output.stderr.is_empty() {
                        &output.stdout
                    } else {
                        &output.stderr
                    }
                )));
            }
        }
        None => {
            ui_state.add_output(OutputLine::Error("Failed to execute uv command.".into()));
        }
    }
}

/// Apply a single pending file action (from edit confirmation flow).
fn apply_pending_action(
    ui_state: &mut ui::UiState,
    app_state: &AppState,
    action: app::PendingAction,
    sandbox: &sandbox::Sandbox,
) {
    let file_ops = project::file_ops::FileOps::new(sandbox, app_state.mode);
    match action {
        app::PendingAction::WriteFile {
            path,
            content,
            diff_preview: _,
        } => {
            let fc = file_ops.prepare_write(&path, &content);
            match fc {
                Ok(fc) => match file_ops.apply_change(&fc) {
                    Ok(()) => {
                        ui_state
                            .add_output(OutputLine::System(format!("Wrote {}", path.display())));
                        run_syntax_check(ui_state, &path, sandbox);
                    }
                    Err(e) => {
                        ui_state.add_output(OutputLine::Error(format!(
                            "Failed to write {}: {}",
                            path.display(),
                            e
                        )));
                    }
                },
                Err(e) => {
                    ui_state.add_output(OutputLine::Error(format!(
                        "Blocked {}: {}",
                        path.display(),
                        e
                    )));
                }
            }
        }
        app::PendingAction::DeleteFile { path } => {
            let fc = file_ops.prepare_delete(&path);
            match fc {
                Ok(fc) => match file_ops.apply_change(&fc) {
                    Ok(()) => {
                        ui_state
                            .add_output(OutputLine::System(format!("Deleted {}", path.display())));
                    }
                    Err(e) => {
                        ui_state.add_output(OutputLine::Error(format!(
                            "Failed to delete {}: {}",
                            path.display(),
                            e
                        )));
                    }
                },
                Err(e) => {
                    ui_state.add_output(OutputLine::Error(format!(
                        "Blocked {}: {}",
                        path.display(),
                        e
                    )));
                }
            }
        }
        app::PendingAction::ExecuteCommand { cmd, args } => {
            ui_state.add_output(OutputLine::System(format!(
                "Skipping command: {} {:?}",
                cmd, args
            )));
        }
    }
}

/// Run syntax check on a file and display results in chat.
fn run_syntax_check(
    ui_state: &mut ui::UiState,
    full_path: &std::path::Path,
    sandbox: &sandbox::Sandbox,
) {
    let path_display = full_path.display().to_string();
    let check_result = std::thread::scope(|s| {
        s.spawn(|| {
            let rt = tokio::runtime::Runtime::new().ok()?;
            rt.block_on(agent::syntax::SyntaxChecker::check(full_path, sandbox))
                .ok()
        })
        .join()
        .ok()?
    });
    if let Some(result) = check_result {
        match result {
            agent::syntax::SyntaxResult::Pass => {
                ui_state.add_output(OutputLine::System(format!("  Syntax OK: {}", path_display)));
            }
            agent::syntax::SyntaxResult::Fail { errors } => {
                ui_state.add_output(OutputLine::Error(format!(
                    "  Syntax error in {}:\n  {}",
                    path_display,
                    errors.lines().take(5).collect::<Vec<_>>().join("\n  ")
                )));
            }
            agent::syntax::SyntaxResult::Skipped(reason) => {
                ui_state.add_output(OutputLine::System(format!(
                    "  Syntax check skipped: {}",
                    reason
                )));
            }
        }
    }
}

fn looks_like_code_request(input: &str) -> bool {
    let lower = input.to_lowercase();
    let code_keywords = [
        "implement",
        "create",
        "write",
        "build",
        "add",
        "refactor",
        "generate",
        "fix",
        "modify",
        "change",
        "update",
        "make a",
        "code",
        "function",
        "class",
        "module",
        "file",
    ];
    code_keywords.iter().any(|k| lower.contains(k))
}
