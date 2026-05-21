mod agent;
mod app;
mod codebase;
mod config;
mod context;
mod logger;
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
use app::{AppMode, AppState, ConversationMessage};
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

    // Initialize file logging (guard must live until app exits)
    let _log_guard = logger::init(&litepilot_dir);

    tracing::info!(
        "LitePilot v{} starting | workspace={} | endpoint={}",
        env!("CARGO_PKG_VERSION"),
        workspace.display(),
        config.ollama_endpoint
    );
    tracing::info!(
        "models: fast={} core={} audit={}",
        config.effective_fast_model(),
        config.core_model,
        config.effective_audit_model()
    );
    tracing::info!("mode: {}", config.default_mode);

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

    tracing::info!("session ended");

    // Restore terminal (even if panic occurred)
    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    match result {
        Ok(inner) => inner,
        Err(panic_payload) => {
            let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            tracing::error!("fatal panic: {}", msg);
            eprintln!("Fatal error: {}", msg);
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
        "Shift+Tab: mode | Enter: send | Shift+Enter: newline | Ctrl+Tab: think | Ctrl+C: quit | /skills: list skills".into(),
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
                    ui_state.stop_thinking();
                    match &r {
                        agent::retry::RetryResult::Success { content, attempts } => {
                            tracing::info!("llm response: {} bytes (attempts={})", content.len(), attempts);
                        }
                        agent::retry::RetryResult::Exhausted { content, attempts, corrections } => {
                            tracing::warn!("llm response exhausted retries: attempts={}, corrections={}", attempts, corrections.len());
                            tracing::info!("llm response content: {} bytes", content.len());
                        }
                        agent::retry::RetryResult::Failed { last_error, attempts } => {
                            tracing::error!("llm failed after {} attempts: {}", attempts, last_error);
                        }
                    }
                    render_retry_result(&mut ui_state, r);
                    ui_state.add_output(OutputLine::Separator);
                }
                agent::retry::PipelineResult::AutoSuccess { changes, applied } => {
                    app_state.is_processing = false;
                    ui_state.stop_thinking();
                    tracing::info!("auto pipeline success: {} files applied", applied.len());
                    render_auto_result(&mut ui_state, &changes, &applied);
                    ui_state.add_output(OutputLine::Separator);
                }
                agent::retry::PipelineResult::AutoFailed { error } => {
                    app_state.is_processing = false;
                    ui_state.stop_thinking();
                    tracing::error!("auto pipeline failed: {}", error);
                    ui_state.add_output(OutputLine::Error(format!("Pipeline failed: {}", error)));
                    ui_state.add_output(OutputLine::Separator);
                }
                agent::retry::PipelineResult::SearchDone { count, .. } => {
                    // Intermediate status — LLM still running
                    tracing::info!("web search: {} results", count);
                    ui_state.add_output(OutputLine::System(format!(
                        "[search] Found {} result(s)",
                        count
                    )));
                    continue; // Don't drain queue yet
                }
                agent::retry::PipelineResult::StreamChunk { content } => {
                    // Remove thinking indicator on first chunk
                    ui_state.stop_thinking();
                    // Append chunk to streaming output — don't mark processing done
                    ui_state.append_stream_chunk(&content);
                    continue;
                }
                agent::retry::PipelineResult::StreamDone { content } => {
                    app_state.is_processing = false;
                    ui_state.stop_thinking();
                    ui_state.finish_stream();
                    tracing::info!("stream complete: {} bytes", content.len());
                    // Record in conversation history
                    if !content.is_empty() {
                        app_state.conversation_history.push(ConversationMessage {
                            role: "assistant".into(),
                            content: content.clone(),
                            tokens: util::text::estimate_tokens(&content),
                        });
                        context::maybe_compact(&mut app_state.conversation_history, &app_state.config.core_model);
                    }
                    let mode = app_state.mode;
                    // Execute bash blocks in Auto mode, show hint in Edit mode
                    if !content.is_empty() {
                        if mode == AppMode::Auto {
                            execute_bash_blocks(&app_state, &mut ui_state, &content);
                        } else if mode == AppMode::Edit {
                            let bash_blocks = parse_bash_blocks(&content);
                            if !bash_blocks.is_empty() {
                                let cmd_count: usize = bash_blocks.iter().map(|b| b.lines().filter(|l| !l.trim().is_empty()).count()).sum();
                                ui_state.add_output(OutputLine::System(format!(
                                    "Detected {} command(s). Use /run <cmd> to execute.",
                                    cmd_count
                                )));
                            }
                        }
                    }
                    // Handle file changes per mode
                    if !content.is_empty() {
                        let changes = agent::AgentPipeline::parse_file_changes(&content);
                        if !changes.is_empty() {
                            match mode {
                                AppMode::Auto => {
                                    auto_apply_changes(&app_state, &mut ui_state, &changes);
                                }
                                AppMode::Edit => {
                                    enter_file_confirmation(&mut app_state, &mut ui_state, &changes);
                                }
                                AppMode::Plan => {
                                    ui_state.add_output(OutputLine::System(format!(
                                        "Detected {} file change(s). Switch to Edit/Auto to apply.",
                                        changes.len()
                                    )));
                                }
                            }
                        }
                    }
                    ui_state.add_output(OutputLine::Separator);
                }
                agent::retry::PipelineResult::PlanReady { plan } => {
                    tracing::info!("plan ready: {} bytes", plan.len());
                    ui_state.stop_thinking();
                    if plan.starts_with("(plan unavailable") {
                        // Plan step failed — proceed without plan
                        ui_state.add_output(OutputLine::System(
                            "Plan step skipped (fast model unavailable). Executing directly...".into(),
                        ));
                        ui_state.start_thinking();
                        spawn_execution_with_plan(
                            &app_state, &ui_state.last_user_input, "",
                            result_tx.clone(),
                        );
                    } else {
                        ui_state.add_output(OutputLine::Plan(plan.clone()));
                        if app_state.mode == AppMode::Auto || app_state.mode == AppMode::Plan {
                            // Auto/Plan mode: auto-execute
                            ui_state.start_thinking();
                            spawn_execution_with_plan(
                                &app_state, &ui_state.last_user_input, &plan,
                                result_tx.clone(),
                            );
                        } else {
                            // Edit mode: wait for approval
                            app_state.pending_plan = Some(plan);
                            app_state.is_processing = false;
                            ui_state.add_output(OutputLine::System(
                                "Enter: execute plan | Esc: cancel".into(),
                            ));
                        }
                    }
                }
                agent::retry::PipelineResult::StepStart { step, total, description } => {
                    ui_state.stop_thinking();
                    tracing::info!("executing step {}/{}: {}", step, total, description);
                    ui_state.add_output(OutputLine::System(format!(
                        "Step {}/{}: {}", step, total, description
                    )));
                    ui_state.start_thinking();
                }
            }

            // Drain next queued message if any
            if !app_state.pending_queue.is_empty() {
                let next = app_state.pending_queue.remove(0);
                ui_state.add_output(OutputLine::System(
                    "Processing queued message...".to_string(),
                ));
                ui_state.add_output(OutputLine::User(next.clone()));
                ui_state.start_thinking();
                spawn_request_for_mode(&app_state, &ollama_client, &next, result_tx.clone());
                app_state.is_processing = true;
            }
        }

        if event::poll(Duration::from_millis(100))? {
            let ev = event::read()?;
            match ev {
                Event::Paste(text) => {
                    ui_state.set_paste(text);
                }
                Event::Key(key) => {
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
                    // Shift+Tab: switch mode (crossterm sends BackTab for Shift+Tab)
                    (_, KeyCode::BackTab) => {
                        let old_mode = app_state.mode;
                        let new_mode = app_state.switch_mode();
                        tracing::info!("mode switch: {} -> {}", old_mode, new_mode);
                        ui_state.add_output(OutputLine::System(format!(
                            "Switched to {} mode",
                            new_mode
                        )));
                    }
                    // Ctrl+C: quit
                    (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                        tracing::info!("user requested quit (Ctrl+C)");
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
                    // Ctrl+Tab: toggle model thinking
                    (KeyModifiers::CONTROL, KeyCode::Tab) => {
                        app_state.think_enabled = !app_state.think_enabled;
                        let state = if app_state.think_enabled {
                            "THINK"
                        } else {
                            "DIRECT"
                        };
                        tracing::info!("think mode: {}", state);
                        ui_state.add_output(OutputLine::System(format!("Model mode: {}", state)));
                    }
                    // Shift+Enter: insert newline
                    (KeyModifiers::SHIFT, KeyCode::Enter) => {
                        ui_state.push_char('\n');
                    }
                    // Enter: submit input or approve pending plan
                    (KeyModifiers::NONE, KeyCode::Enter) => {
                        // Plan approval: empty input + pending plan = approve
                        if ui_state.input_text.is_empty() {
                            if let Some(plan) = app_state.pending_plan.take() {
                                ui_state.add_output(OutputLine::System("Executing plan...".into()));
                                ui_state.start_thinking();
                                app_state.is_processing = true;
                                spawn_execution_with_plan(
                                    &app_state, &ui_state.last_user_input, &plan,
                                    result_tx.clone(),
                                );
                                continue;
                            }
                        }

                        let input = ui_state.take_input();
                        if !input.is_empty() {
                            app_state.input_history.push(input.clone());
                            app_state.history_index = 0;
                            tracing::info!("user input: {}", input.trim());
                            ui_state.last_user_input = input.clone();

                            if input.trim() == "/quit" || input.trim() == "/exit" {
                                tracing::info!("session ending via /{}", input.trim().trim_start_matches('/'));
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
                                tracing::info!("running setup wizard");
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
                                tracing::info!("applying file changes, mode={}", app_state.mode);
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
                            } else if let Some(run_cmd) = trimmed.strip_prefix("/run ") {
                                handle_run_command(&mut app_state, &mut ui_state, run_cmd.trim());
                            } else if let Some(cmd) = trimmed.strip_prefix('/') {
                                let (skill_name, args) = match cmd.split_once(' ') {
                                    Some((name, rest)) => (name, rest.trim()),
                                    None => (cmd, ""),
                                };

                                if let Some(skill) = app_state.skills.get(skill_name).cloned() {
                                    tracing::info!("skill: /{} {:?}", skill_name, args);
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
                                        ui_state.start_thinking();
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
                                // Record in conversation history
                                app_state.conversation_history.push(ConversationMessage {
                                    role: "user".into(),
                                    content: input.clone(),
                                    tokens: util::text::estimate_tokens(&input),
                                });
                                ui_state.start_thinking();
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
                    // Escape: cancel pending plan or scroll to bottom
                    (KeyModifiers::NONE, KeyCode::Esc) => {
                        if app_state.pending_plan.take().is_some() {
                            ui_state.add_output(OutputLine::System("Plan cancelled.".into()));
                        } else {
                            ui_state.scroll_to_bottom();
                        }
                    }
                    // Page Up: scroll chat up
                    (KeyModifiers::NONE, KeyCode::PageUp) => {
                        ui_state.scroll_up(10);
                    }
                    // Page Down: scroll chat down
                    (KeyModifiers::NONE, KeyCode::PageDown) => {
                        ui_state.scroll_down(10);
                    }
                    // Character input (allow NONE or SHIFT for uppercase)
                    (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                        app_state.history_index = 0;
                        ui_state.push_char(c);
                    }
                    // Up arrow: navigate input history (older)
                    (KeyModifiers::NONE, KeyCode::Up) => {
                        if !app_state.input_history.is_empty() && app_state.history_index < app_state.input_history.len() {
                            app_state.history_index += 1;
                            let idx = app_state.input_history.len() - app_state.history_index;
                            ui_state.input_text = app_state.input_history[idx].clone();
                            ui_state.input_cursor = ui_state.input_text.chars().count();
                        }
                    }
                    // Down arrow: navigate input history (newer)
                    (KeyModifiers::NONE, KeyCode::Down) => {
                        if app_state.history_index > 0 {
                            app_state.history_index -= 1;
                            if app_state.history_index == 0 {
                                ui_state.input_text.clear();
                                ui_state.input_cursor = 0;
                            } else {
                                let idx = app_state.input_history.len() - app_state.history_index;
                                ui_state.input_text = app_state.input_history[idx].clone();
                                ui_state.input_cursor = ui_state.input_text.chars().count();
                            }
                        }
                    }
                    _ => {}
                }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/// Spawn a streaming chat request on a background thread (direct, no plan step).
#[allow(dead_code)]
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
    let context_window_limit = app_state.config.context_window_limit;
    let input = input.to_string();
    let web_search_enabled = app_state.web_search_enabled;
    let max_search_tokens = app_state.config.max_search_context_tokens;
    let max_file_lines = app_state.config.max_file_lines;
    let think = app_state.think_enabled;
    let system_with_workspace = format!(
        "{}\n\nWorking directory: {}\nCurrent date: {}",
        apply_prompt_limits(agent::prompts::CODING_SYSTEM, max_file_lines),
        app_state.workspace.display(),
        current_datetime(),
    );

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

        // Build streaming request — use a client without overall deadline
        let http = match ollama::OllamaClient::streaming_http_client(Duration::from_secs(connect_timeout)) {
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
        let messages = vec![
            ollama::chat::ChatMessage::system(&system_with_workspace),
            ollama::chat::ChatMessage::user(&user_message),
        ];
        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

        let stream = ollama::OllamaClient::chat_stream(http, endpoint, model, messages, think, context_window_limit, cancel_rx);

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

/// Plan-then-execute: runs a quick plan step on the fast model, sends PlanReady,
/// then the main loop waits for user approval before calling spawn_execution_with_plan.
fn spawn_plan_then_execute(
    app_state: &AppState,
    _client: &ollama::OllamaClient,
    input: &str,
    tx: mpsc::Sender<agent::retry::PipelineResult>,
) {
    let fast_model = app_state.config.effective_fast_model().to_string();
    let endpoint = app_state.config.ollama_endpoint.clone();
    let connect_timeout = app_state.config.connect_timeout;
    let max_file_lines = app_state.config.max_file_lines;
    let input = input.to_string();
    let history_ctx = app_state
        .conversation_history
        .iter()
        .rev()
        .take(6)
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");
    let plan_mode = app_state.mode == AppMode::Plan;
    let workspace = app_state.workspace.clone();

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
            fast_model: fast_model.clone(),
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

        // Quick plan step using fast model
        let project_listing = std::fs::read_dir(&workspace)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        let user_msg = format!(
            "Working directory: {}\nFiles in directory: {}\nCurrent date: {}\n\n{}\nNew request: {}",
            workspace.display(),
            project_listing,
            current_datetime(),
            if history_ctx.is_empty() {
                String::new()
            } else {
                format!("Recent context:\n{}\n\n", history_ctx)
            },
            input
        );
        let messages = vec![
            ollama::chat::ChatMessage::system(apply_prompt_limits(agent::prompts::QUICK_PLAN_SYSTEM, max_file_lines)),
            ollama::chat::ChatMessage::user(&user_msg),
        ];

        match rt.block_on(bg_client.chat(&fast_model, messages, false)) {
            Ok(resp) => {
                let plan = resp.content;
                let _ = tx.send(agent::retry::PipelineResult::PlanReady {
                    plan: plan.clone(),
                });
                // In Plan mode, auto-continue to execution (read-only analysis)
                if plan_mode {
                    // The main loop will handle this: Plan mode skips the approval gate
                }
            }
            Err(e) => {
                // Plan step failed — fall back to direct streaming without plan
                let _ = tx.send(agent::retry::PipelineResult::PlanReady {
                    plan: format!("(plan unavailable: {})", e),
                });
            }
        }
    });
}

/// Execute with a plan: step-by-step execution, one plan step per LLM call.
fn spawn_execution_with_plan(
    app_state: &AppState,
    input: &str,
    plan: &str,
    tx: mpsc::Sender<agent::retry::PipelineResult>,
) {
    let model = app_state.config.core_model.clone();
    if model.is_empty() {
        let _ = tx.send(agent::retry::PipelineResult::Retry(
            agent::retry::RetryResult::Failed {
                last_error: "No core model configured.".into(),
                attempts: 0,
            },
        ));
        return;
    }

    let endpoint = app_state.config.ollama_endpoint.clone();
    let connect_timeout = app_state.config.connect_timeout;
    let input = input.to_string();
    let think = app_state.think_enabled;
    let plan = plan.to_string();
    let core_model = app_state.config.core_model.clone();
    let history = app_state.conversation_history.clone();
    let workspace = app_state.workspace.clone();
    let context_window_limit = app_state.config.context_window_limit;
    let max_file_lines = app_state.config.max_file_lines;
    let now = current_datetime();
    let coding_system = apply_prompt_limits(agent::prompts::CODING_SYSTEM, max_file_lines);

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

        let http = match ollama::OllamaClient::streaming_http_client(
            std::time::Duration::from_secs(connect_timeout),
        ) {
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

        // Parse plan into individual steps
        let steps: Vec<String> = plan
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() { return None; }
                // Match numbered steps: "1. ..." or "1) ..."
                let step = trimmed
                    .strip_prefix(|c: char| c.is_ascii_digit())
                    .and_then(|s| s.strip_prefix(|c: char| c.is_ascii_digit()))
                    .and_then(|s| s.strip_prefix('.').or_else(|| s.strip_prefix(')')))
                    .map(|s| s.trim().to_string());
                step.filter(|s| !s.is_empty())
            })
            .collect();

        if steps.is_empty() {
            tracing::info!("no parseable steps in plan, executing as single request (plan: {} bytes)", plan.len());
            // No parseable steps — execute as a single request
            let output_budget = context_window_limit / 4;
            let system_prompt = format!(
                "{}\n\nWorking directory: {}\nCurrent date: {}\nOutput limit: keep response under {} tokens ({} lines max). If the task is too large, output only the first part and note what remains.",
                coding_system,
                workspace.display(),
                now,
                output_budget,
                output_budget / 4,
            );
            let messages = context::build_messages(
                &system_prompt, &history, &input, Some(&plan), &core_model,
            );
            let content = stream_single_step(&rt, http, &endpoint, &model, messages, think, context_window_limit, &tx);
            let _ = tx.send(agent::retry::PipelineResult::StreamDone { content });
            return;
        }

        let total_steps = steps.len();
        tracing::info!("executing {} steps from plan", total_steps);
        for (i, s) in steps.iter().enumerate() {
            tracing::info!("  step {}: {}", i + 1, s);
        }
        let mut all_content = String::new();
        let mut step_results: Vec<String> = Vec::new();
        let per_step_budget = context_window_limit / (4 * total_steps.max(1) as u64);
        let system_prompt = format!(
            "{}\n\nWorking directory: {}\nCurrent date: {}\nOutput limit for this step: keep response under {} tokens ({} lines max). Output ONLY the current step — do not attempt other steps.",
            coding_system,
            workspace.display(),
            now,
            per_step_budget,
            per_step_budget / 4,
        );

        for (i, step_desc) in steps.iter().enumerate() {
            let step_num = i + 1;

            // Notify main loop which step is starting
            let _ = tx.send(agent::retry::PipelineResult::StepStart {
                step: step_num,
                total: total_steps,
                description: step_desc.clone(),
            });

            // Build per-step context: plan overview + previous results + current step
            let prev_summary = if step_results.is_empty() {
                String::new()
            } else {
                let mut s = "\n\n[Previous steps completed]\n".to_string();
                for (j, result) in step_results.iter().enumerate() {
                    let preview: String = result.chars().take(300).collect();
                    s.push_str(&format!("Step {}: {}...\n", j + 1, preview));
                }
                s
            };

            let user_msg = format!(
                "Original request: {}\n\nPlan:\n{}\n{}Now execute ONLY step {} of {}: {}",
                input,
                plan,
                prev_summary,
                step_num,
                total_steps,
                step_desc,
            );

            // Build messages with history
            let mut messages = context::build_messages(
                &system_prompt, &history, &input, None, &core_model,
            );
            // Override the user message with the step-specific one
            if let Some(last) = messages.last_mut() {
                last.content = user_msg;
            }

            let step_content = stream_single_step(&rt, http.clone(), &endpoint, &model, messages, think, context_window_limit, &tx);

            if step_content.starts_with("ERROR:") {
                let _ = tx.send(agent::retry::PipelineResult::Retry(
                    agent::retry::RetryResult::Failed {
                        last_error: step_content,
                        attempts: step_num,
                    },
                ));
                return;
            }

            all_content.push_str(&step_content);
            all_content.push('\n');
            step_results.push(step_content);
        }

        let _ = tx.send(agent::retry::PipelineResult::StreamDone { content: all_content });
    });
}

/// Stream a single step's LLM call, returning the accumulated content.
/// On error, returns a string starting with "ERROR:".
fn stream_single_step(
    rt: &tokio::runtime::Runtime,
    http: reqwest::Client,
    endpoint: &str,
    model: &str,
    messages: Vec<ollama::chat::ChatMessage>,
    think: bool,
    num_ctx: u64,
    tx: &mpsc::Sender<agent::retry::PipelineResult>,
) -> String {
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    let stream = ollama::OllamaClient::chat_stream(
        http, endpoint.to_string(), model.to_string(), messages, think, num_ctx, cancel_rx,
    );
    let mut pin = std::pin::pin!(stream);
    let tx = tx.clone();

    let result: Result<String, String> = rt.block_on(async {
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

    drop(cancel_tx);

    match result {
        Ok(content) => content,
        Err(e) => format!("ERROR: {}", e),
    }
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

    let system_prompt = format!(
        "{}\n\nWorking directory: {}\nCurrent date: {}\n\n{}",
        apply_prompt_limits(agent::prompts::CODING_SYSTEM, app_state.config.max_file_lines),
        app_state.workspace.display(),
        current_datetime(),
        skill.content
    );
    let max_retries = app_state.config.max_retries;
    let endpoint = app_state.config.ollama_endpoint.clone();
    let connect_timeout = app_state.config.connect_timeout;
    let user_message = args.to_string();
    let kind = if skill.name == "simplify" || skill.name == "review" || skill.name == "test" {
        agent::retry::ResponseKind::CodeImplementation
    } else {
        agent::retry::ResponseKind::Chat
    };
    let think = app_state.think_enabled;

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
            think,
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

/// Route request based on current mode:
/// - Auto + code keywords → full auto pipeline (plan→implement→audit)
/// - Edit/Auto (non-code) → plan-then-execute (plan step, then streaming execution)
/// - Plan mode → plan-then-execute (plan shown as analysis, no approval)
fn spawn_request_for_mode(
    app_state: &AppState,
    client: &ollama::OllamaClient,
    input: &str,
    tx: mpsc::Sender<agent::retry::PipelineResult>,
) {
    if app_state.mode == AppMode::Auto && looks_like_code_request(input) {
        spawn_auto_pipeline(app_state, input, tx);
    } else {
        spawn_plan_then_execute(app_state, client, input, tx);
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

/// Handle /run <command> — execute a shell command via the sandbox.
/// Returns current date and time as a human-readable string for LLM context.
fn current_datetime() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M %A").to_string()
}

/// Replace {MAX_LINES} placeholder in prompts with the configured limit.
fn apply_prompt_limits(prompt: &str, max_file_lines: usize) -> String {
    prompt.replace("{MAX_LINES}", &max_file_lines.to_string())
}

fn handle_run_command(app_state: &mut AppState, ui_state: &mut ui::UiState, cmd: &str) {
    if cmd.is_empty() {
        ui_state.add_output(OutputLine::Error("Usage: /run <command>".into()));
        return;
    }

    if !app_state.mode.can_execute_command() {
        ui_state.add_output(OutputLine::Error(
            "Command execution not allowed in Plan mode.".into(),
        ));
        return;
    }

    // Parse command into binary + args
    let parts: Vec<String> = cmd.split_whitespace().map(String::from).collect();
    if parts.is_empty() {
        ui_state.add_output(OutputLine::Error("Empty command.".into()));
        return;
    }

    let sandbox = sandbox::Sandbox::new(app_state.workspace.clone());
    let executor = sandbox::executor::Executor::new(&sandbox);

    let bin = parts[0].clone();
    let args: Vec<String> = parts[1..].to_vec();

    // Validate command against sandbox rules
    if let Err(e) = sandbox.validate_command(&bin, &args) {
        ui_state.add_output(OutputLine::Error(format!("Blocked: {}", e)));
        return;
    }

    tracing::info!("run: {} {}", bin, args.join(" "));
    ui_state.add_output(OutputLine::System(format!("Running: {} {}", bin, args.join(" "))));

    let result = std::thread::scope(|s| {
        s.spawn(|| {
            let rt = tokio::runtime::Runtime::new().ok()?;
            rt.block_on(executor.run(&bin, &args, None)).ok()
        })
        .join()
        .ok()?
    });

    match result {
        Some(output) => {
            if output.success {
                if !output.stdout.is_empty() {
                    ui_state.add_output(OutputLine::System(output.stdout.trim_end().to_string()));
                }
                ui_state.add_output(OutputLine::System("Done.".into()));
            } else {
                ui_state.add_output(OutputLine::Error(format!(
                    "Exit {}: {}",
                    output.exit_code.unwrap_or(1),
                    if output.stderr.is_empty() {
                        output.stdout.trim_end()
                    } else {
                        output.stderr.trim_end()
                    }
                )));
            }
        }
        None => {
            ui_state.add_output(OutputLine::Error("Failed to execute command.".into()));
        }
    }
}

/// Parse ```bash ... ``` blocks from LLM response content.
fn parse_bash_blocks(content: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut in_bash = false;
    let mut buffer = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if in_bash {
            if trimmed == "```" {
                in_bash = false;
                let cmds = buffer.trim().to_string();
                if !cmds.is_empty() {
                    commands.push(cmds);
                }
                buffer.clear();
            } else {
                // Skip comment-only lines
                let stripped = trimmed.trim_start_matches('#').trim();
                if !stripped.is_empty() && !trimmed.starts_with('#') {
                    buffer.push_str(trimmed);
                    buffer.push('\n');
                }
            }
        } else if trimmed == "```bash" || trimmed == "```sh" || trimmed == "```shell" {
            in_bash = true;
            buffer.clear();
        }
    }

    commands
}

/// Execute bash blocks extracted from the LLM response.
fn execute_bash_blocks(
    app_state: &AppState,
    ui_state: &mut ui::UiState,
    content: &str,
) {
    let blocks = parse_bash_blocks(content);
    if blocks.is_empty() {
        return;
    }

    let sandbox = sandbox::Sandbox::new(app_state.workspace.clone());
    let executor = sandbox::executor::Executor::new(&sandbox);

    for block in &blocks {
        // Each block may contain multiple commands (one per line)
        for cmd_line in block.lines() {
            let cmd_line = cmd_line.trim();
            if cmd_line.is_empty() {
                continue;
            }

            let parts: Vec<String> = cmd_line.split_whitespace().map(String::from).collect();
            if parts.is_empty() {
                continue;
            }

            let bin = parts[0].clone();
            let args: Vec<String> = parts[1..].to_vec();

            // Validate against sandbox rules
            if let Err(e) = sandbox.validate_command(&bin, &args) {
                ui_state.add_output(OutputLine::Error(format!("Blocked: {} — {}", cmd_line, e)));
                continue;
            }

            ui_state.add_output(OutputLine::System(format!("$ {}", cmd_line)));

            let result = std::thread::scope(|s| {
                s.spawn(|| {
                    let rt = tokio::runtime::Runtime::new().ok()?;
                    rt.block_on(executor.run(&bin, &args, None)).ok()
                })
                .join()
                .ok()?
            });

            match result {
                Some(output) => {
                    if output.success {
                        let stdout = output.stdout.trim_end();
                        if !stdout.is_empty() {
                            ui_state.add_output(OutputLine::System(stdout.to_string()));
                        }
                    } else {
                        ui_state.add_output(OutputLine::Error(format!(
                            "Exit {}: {}",
                            output.exit_code.unwrap_or(1),
                            if output.stderr.is_empty() {
                                output.stdout.trim_end()
                            } else {
                                output.stderr.trim_end()
                            }
                        )));
                    }
                }
                None => {
                    ui_state.add_output(OutputLine::Error(format!("Failed: {}", cmd_line)));
                }
            }
        }
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
/// Auto-apply all file changes (Auto mode — no confirmation needed).
fn auto_apply_changes(
    app_state: &AppState,
    ui_state: &mut ui::UiState,
    changes: &[agent::FileChange],
) {
    let sandbox = sandbox::Sandbox::new(app_state.workspace.clone());
    let file_ops = project::file_ops::FileOps::new(&sandbox, app_state.mode);

    for change in changes {
        let full_path = app_state.workspace.join(&change.path);
        let fc = match if change.action == "delete" {
            file_ops.prepare_delete(&full_path)
        } else {
            file_ops.prepare_write(&full_path, &change.content)
        } {
            Ok(fc) => fc,
            Err(e) => {
                ui_state.add_output(OutputLine::Error(format!(
                    "Blocked {}: {}",
                    change.path.display(),
                    e
                )));
                continue;
            }
        };

        match file_ops.apply_change(&fc) {
            Ok(()) => {
                ui_state.add_output(OutputLine::System(format!(
                    "{} {}",
                    if change.action == "delete" { "Deleted" } else { "Wrote" },
                    change.path.display()
                )));
                if change.action != "delete" {
                    run_syntax_check(ui_state, &full_path, &sandbox);
                }
            }
            Err(e) => {
                ui_state.add_output(OutputLine::Error(format!(
                    "Failed {} {}: {}",
                    change.action,
                    change.path.display(),
                    e
                )));
            }
        }
    }
}

/// Enter file-by-file confirmation flow (Edit mode — y/n/a).
fn enter_file_confirmation(
    app_state: &mut AppState,
    ui_state: &mut ui::UiState,
    changes: &[agent::FileChange],
) {
    let sandbox = sandbox::Sandbox::new(app_state.workspace.clone());
    let file_ops = project::file_ops::FileOps::new(&sandbox, app_state.mode);

    app_state.clear_pending();
    let mut queued = 0;

    for change in changes {
        let full_path = app_state.workspace.join(&change.path);
        let fc = match if change.action == "delete" {
            file_ops.prepare_delete(&full_path)
        } else {
            file_ops.prepare_write(&full_path, &change.content)
        } {
            Ok(fc) => fc,
            Err(e) => {
                ui_state.add_output(OutputLine::Error(format!(
                    "Blocked {}: {}",
                    change.path.display(),
                    e
                )));
                continue;
            }
        };

        // Show diff preview
        if !fc.diff_preview.is_empty() {
            ui_state.add_output(OutputLine::System(format!(
                "--- {} ({})",
                change.path.display(),
                change.action
            )));
            for line in fc.diff_preview.lines() {
                if line.starts_with('-') && !line.starts_with("---") {
                    ui_state.add_output(OutputLine::Diff {
                        added: vec![],
                        removed: vec![line.to_string()],
                    });
                } else if line.starts_with('+') && !line.starts_with("+++") {
                    ui_state.add_output(OutputLine::Diff {
                        added: vec![line.to_string()],
                        removed: vec![],
                    });
                }
            }
        } else {
            ui_state.add_output(OutputLine::System(format!(
                "{} {}",
                change.action,
                change.path.display()
            )));
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
            "Apply {} file(s)? y/n/a (y=yes, n=no, a=apply all):",
            queued
        )));
    }
}

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
