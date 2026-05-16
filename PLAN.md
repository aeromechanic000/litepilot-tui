# LiteCode-TUI Implementation Plan

## Overview

This plan breaks the IDEA.md spec into ~20 sequential milestones grouped by
dependency order. Each milestone is a vertical slice: implement, write tests,
verify green. The plan targets **v1.0** first (stable TUI + Ollama + basic
agent), then v1.1–v1.3 feature increments.

All milestones include specific test requirements. Tests run via `cargo test`.
Integration tests needing a live Ollama instance are gated behind `#[ignore]`.

---

## Phase 0 — Project Bootstrap

### M0.1 Cargo project + CI skeleton

**Create:**
- `Cargo.toml` with all dependencies (ratatui, crossterm, tokio, reqwest,
  serde, toml, walkdir, similar, insta, wiremock, proptest, tempfile,
  clap, anyhow, tracing, tracing-subscriber)
- `src/main.rs` — minimal `fn main()` that prints version and exits
- `.github/workflows/ci.yml` — fmt, clippy, test on linux/mac/windows

**Auto-test:**
- `cargo build` succeeds
- `cargo test` runs (0 tests, no failures)
- `cargo fmt --check` passes
- `cargo clippy` passes with no warnings

---

## Phase 1 — Foundation Layer

### M1.1 Config module (`src/config.rs`)

**Implement:**
- `Config` struct with serde derive: `ollama_endpoint`, `connect_timeout`,
  `fast_model`, `core_model`, `audit_model`, `code_base_path`,
  `default_mode`, `enable_auto_syntax_check`, `prefer_uv_toolchain`,
  `auto_run_after_fix`, `enable_free_web_search`,
  `auto_switch_network_region`, `search_cache_valid_days`,
  `max_search_context_tokens`
- `load(path) -> Result<Config>` — read TOML, deserialize, validate
- `save(path, &Config) -> Result<()>` — serialize to TOML
- `default()` matching IDEA.md section 9 defaults
- `validate() -> Result<()>` — check endpoint format, model names non-empty

**Auto-test (unit):**
- Parse valid TOML → Config struct round-trips correctly
- Missing fields fall back to defaults
- Invalid endpoint URL returns validation error
- Extra unknown fields are ignored (serde deny_unknown_fields = false)
- proptest: generate random valid configs, serialize/parse round-trip

### M1.2 First-run wizard (`src/config.rs` + `src/ui/setup.rs`)

**Implement:**
- `ensure_config_dir() -> Result<PathBuf>` — create `~/.litecode/` tree
  (`sessions/`, `cache/`, `code_base/`)
- `first_run_setup(terminal) -> Result<Config>` — interactive wizard:
  1. Prompt Ollama endpoint (default `http://127.0.0.1:11434`)
  2. Test connectivity (see M1.3)
  3. Fetch model list (see M1.3)
  4. User selects fast/core/audit models
  5. Save config, return Config
- Detect if `~/.litecode/config.toml` exists; if not, trigger wizard on startup

**Auto-test (unit):**
- `ensure_config_dir` creates correct directory tree in a temp dir
- Wizard skips if config.toml already exists
- Wizard produces a valid Config after simulated input

### M1.3 Ollama client core (`src/ollama/mod.rs`, `src/ollama/model.rs`)

**Implement:**
- `OllamaClient::new(endpoint: String, timeout: Duration)`
- `ping() -> Result<()>` — GET `/api/tags` or HEAD check
- `list_models() -> Result<Vec<ModelInfo>>` — GET `/api/tags`, parse response
- `ModelInfo` struct: name, size, parameter_count, quantization_level,
  context_window (parsed from model name heuristics or modelfile)
- `ModelSize` enum: Small (≤5B), Medium (6-14B), Large (≥30B)
- `classify_model(model_info) -> ModelSize`

**Auto-test (unit with wiremock):**
- Mock `/api/tags` response → parse model list correctly
- Mock connection refused → `ping()` returns clear error
- Mock timeout → returns timeout error
- Model size classification: test boundary cases (5B, 6B, 14B, 30B)
- Empty model list handled gracefully

### M1.4 Ollama streaming chat (`src/ollama/chat.rs`)

**Implement:**
- `ChatRequest` struct: model, messages (role+content), stream: bool, options
- `stream_chat(client, request) -> impl Stream<Item = Result<ChatChunk>>`
- Parse newline-delimited JSON streaming response from `/api/chat`
- Handle `done: true` terminator
- Cancellation via `CancellationToken`
- Retry logic: up to 3 retries on connection errors with backoff

**Auto-test (unit with wiremock):**
- Mock streaming response → collect all chunks, verify content
- Mock mid-stream disconnect → retry succeeds
- Mock model not found error → returns structured error
- Empty response stream handled
- Cancellation token stops mid-stream

---

## Phase 2 — Application State & Modes

### M2.1 App state machine (`src/app.rs`)

**Implement:**
- `AppMode` enum: Plan, Edit, Auto
- `AppState` struct: current_mode, config, project_path, ollama_client,
  chat_history, file_tree, active_panel
- `ModeSwitch` behavior: `Shift+Tab` cycles Plan→Edit→Auto→Plan
- Mode-specific permission checks:
  - `can_write_file(mode) -> bool`
  - `can_execute_command(mode) -> bool`
  - `needs_confirmation(mode) -> bool`

**Auto-test (unit):**
- Mode cycle: Plan→Edit→Auto→Plan round-trip
- Permission checks correct per mode
- State transitions preserve existing data (chat history, config)

### M2.2 Session persistence (`src/session/mod.rs`, `src/session/persistence.rs`)

**Implement:**
- `Session` struct: id, created_at, messages, mode_history
- `save_session(session, dir) -> Result<()>` — JSON to `~/.litecode/sessions/`
- `load_session(id, dir) -> Result<Session>`
- `list_sessions(dir) -> Result<Vec<SessionMeta>>` — sorted by date
- Auto-save on each assistant response completion

**Auto-test (unit):**
- Save/load round-trip preserves all fields
- Corrupted session file returns error, doesn't crash
- Many sessions listed in correct order
- Concurrent save doesn't corrupt (use atomic write)

---

## Phase 3 — Agent Pipeline

### M3.1 Agent orchestrator (`src/agent/mod.rs`)

**Implement:**
- `AgentPipeline` struct holding references to ollama_client, config,
  codebase, sandbox
- `run_planning(user_request, context) -> Result<Plan>` — calls fast_model
- `run_implementation(plan, context) -> Result<Vec<FileChange>>` — calls core_model
- `run_audit(changes, context) -> Result<AuditResult>` — calls audit_model
- Pipeline modes:
  - Plan mode: only `run_planning`
  - Edit mode: planning + implementation → diff preview → await user
  - Auto mode: full pipeline + auto-apply if audit passes

**Auto-test (unit with mocked Ollama):**
- Plan mode only calls fast_model
- Edit mode calls fast + core, returns diffs for confirmation
- Auto mode calls all three, auto-applies passing audit
- Audit failure triggers re-implementation (max 2 retries)

### M3.2 Prompt engineering (`src/agent/prompts.rs` — new file)

**Implement:**
- System prompts for each model tier:
  - Fast: planning-focused, structured output (markdown plan)
  - Core: coding-focused, file-structured output
  - Audit: review-focused, issues list format
- Model-size-adaptive prompt templates:
  - Small models: explicit step-by-step instructions, reference code_base
  - Medium models: standard instructions
  - Large models: minimal instructions, rely on model capability
- Context window management: truncate chat history to fit model context

**Auto-test (unit):**
- Small model prompt includes code_base references
- Large model prompt is more concise
- Context truncation preserves recent messages
- System prompt is always first in message array

### M3.3 Syntax checker (`src/agent/syntax.rs`)

**Implement:**
- `SyntaxChecker` struct with command map per language
- `check(file_path) -> Result<SyntaxResult>`
- Language detection by extension
- Command map: Python→py_compile, JS→node -c, Bash→bash -n,
  Rust→rustc --check, Go→go vet, C/C++→gcc -fsyntax-only
- Capture stdout/stderr, parse error locations

**Auto-test (integration, local commands):**
- Valid Python file → passes
- Python with syntax error → returns line number and message
- Unsupported language → returns Skipped
- File not found → returns error

---

## Phase 4 — Sandbox & File Operations

### M4.1 Sandbox core (`src/sandbox/mod.rs`, `src/sandbox/executor.rs`)

**Implement:**
- `Sandbox::new(workspace_root: PathBuf)`
- `validate_path(&self, path: &Path) -> Result<CanonicalPath>` — canonicalize,
  ensure within workspace_root, reject `..` traversal
- `validate_command(&self, cmd: &str, args: &[String]) -> Result<()>` —
  allowlist: build tools, interpreters, package managers, git, curl, ls, cat
- Blocklist: rm -rf /, chmod 777, sudo, mkfs, dd, format, del /s
- `execute(&self, cmd: &str, args: &[String]) -> Result<CommandOutput>`

**Auto-test (unit):**
- Path within workspace → allowed
- Path with `..` escaping workspace → rejected
- Symlink pointing outside workspace → rejected
- Allowed commands: cargo, python, node, uv, npm, git, curl
- Blocked commands: sudo, rm -rf /, chmod 777, mkfs
- Auto mode enforcement, Plan mode blocks all execution

### M4.2 Project file operations (`src/project/mod.rs`, `src/project/file_ops.rs`)

**Implement:**
- `ProjectContext::new(root: PathBuf)` — scan file tree (respect .gitignore)
- `read_file(&self, path) -> Result<String>` — with sandbox path check
- `write_file(&self, path, content) -> Result<()>` — Edit mode: stage diff,
  await confirmation; Auto mode: write directly if sandbox passes
- `delete_file(&self, path) -> Result<()>` — same mode logic
- `list_tree(&self) -> Vec<FileEntry>` — for sidebar display

**Auto-test (unit with temp dirs):**
- Read existing file → returns content
- Write new file in workspace → succeeds
- Write file outside workspace → rejected
- Delete file in workspace → succeeds (Edit mode confirms first)
- .gitignore entries excluded from tree

### M4.3 UV toolchain integration (`src/project/uv.rs`)

**Implement:**
- `UvManager` struct
- `init(path) -> Result<()>` — run `uv init`
- `create_venv(path) -> Result<()>` — run `uv venv`
- `add(path, package) -> Result<()>` — run `uv add <package>`
- `run(path, script) -> Result<Output>` — run `uv run <script>`
- Detect if `uv` is available on PATH, provide helpful error if not

**Auto-test (integration, requires uv installed):**
- `uv init` creates pyproject.toml in temp dir
- `uv add` updates dependencies
- Missing `uv` binary returns clear error message

---

## Phase 5 — TUI Rendering

### M5.1 Theme & layout primitives (`src/ui/theme.rs`, `src/ui/layout.rs`)

**Implement:**
- `Theme` struct with all IDEA.md colors: DeepSpaceBlue(#165DFF),
  FogBlue(#4080FF), CharcoalGray(#1E2228), DarkBlueGray(#232733), etc.
- `AppLayout` struct defining ratatui layout rects:
  - Top status bar (fixed height 1)
  - Left sidebar (width 30%, collapsible)
  - Center main panel (flex)
  - Bottom input area (fixed height 3)
- Rendering functions for each area

**Auto-test (snapshot with insta):**
- Render empty layout → snapshot
- Render with each mode indicator → snapshot
- Render collapsed sidebar → snapshot
- Test on 80x24 and 120x40 terminal sizes

### M5.2 Status bar (`src/ui/layout.rs` extend)

**Implement:**
- Display: "LiteCode" logo, Ollama endpoint, three model names,
  current mode badge, working directory, search on/off indicator
- Color-coded: connected=blue, disconnected=red, mode=themed

**Auto-test (snapshot):**
- All info visible in 120-col terminal
- Truncation logic for narrow terminals (80-col)
- Disconnected state rendering

### M5.3 Chat panel (`src/ui/chat.rs`)

**Implement:**
- Render streaming tokens as they arrive (append-only scroll)
- Render code blocks with syntax highlighting (syntect or simple regex)
- Render diff views (green/red for add/remove)
- Render plan documents (markdown-like formatting)
- Auto-scroll with manual scroll-up support (Page Up/Down, mouse wheel)

**Auto-test (snapshot):**
- Short text message rendering
- Code block with highlighting
- Diff view (added/removed lines)
- Long message with scrolling state

### M5.4 Sidebar (`src/ui/sidebar.rs`)

**Implement:**
- Two sections: project file tree + code_base browser
- Tab/Shift-Tab to switch between sections
- Collapse/expand directories (Enter or arrow keys)
- In code_base: show `@LIGHT_DESC` preview, insert reference into context

**Auto-test (snapshot):**
- Collapsed tree view
- Expanded tree with file icons
- code_base section with description preview

### M5.5 Input bar (`src/ui/input.rs`)

**Implement:**
- Multi-line text input (Enter submits, Shift+Enter for newline — or config)
- Show keybinding hints below input
- Handle paste events
- Command history (Up/Down arrows)

**Auto-test (snapshot):**
- Empty input with hints
- Input with long text (wrapping)
- Multi-line input state

### M5.6 Event loop (`src/app.rs` extend, `src/main.rs` extend)

**Implement:**
- Main tokio event loop: crossterm events → app state updates → re-render
- Key routing:
  - `Shift+Tab`: mode switch
  - `Ctrl+C` / `q`: quit (confirm in Auto mode)
  - `Enter`: submit input / confirm action
  - `Escape`: cancel / deselect
  - Arrow keys: navigate sidebar/scroll
  - `Ctrl+S`: toggle web search
- Async bridge: user input → agent pipeline (spawn tokio task) → stream
  results back to UI via `tokio::sync::mpsc`

**Auto-test (unit):**
- Key events produce correct state transitions
- Quit confirmation in Auto mode
- Input submission triggers agent pipeline

---

## Phase 6 — CodeBase & Search

### M6.1 Built-in code templates (`src/codebase/mod.rs`, `src/codebase/index.rs`)

**Implement:**
- Embed template files at compile time via `include_dir!` or load from
  `~/.litecode/code_base/`
- Parse `@LIGHT_DESC`, `@LIGHT_SCENE`, `@LIGHT_TAGS` headers
- `search(query, tags) -> Vec<TemplateMatch>` — tag-based matching
- `load_template(name) -> Result<String>`

**Auto-test (unit):**
- Parse tags from template headers
- Search by tag returns correct templates
- Search by description substring matches
- Empty query returns nothing

### M6.2 Web search (`src/search/mod.rs`, `src/search/cache.rs`)

**Implement:**
- Region detection: try multiple search endpoints, use fastest responding
- Scrape public search results: extract title + snippet + URL
- Fetch top result pages: extract text content, strip ads/nav
- Truncate results to `max_search_context_tokens`
- Cache to `~/.litecode/cache/web_search/` with TTL
- On/Off toggle in status bar and config

**Auto-test (unit with wiremock):**
- Mock search results → parse correctly
- Cache hit returns cached result without HTTP call
- Cache expiry triggers fresh fetch
- Truncation respects token limit
- Region fallback when primary source fails

---

## Phase 7 — Diff & Edit Flow

### M7.1 Diff generation & display (`src/util/diff.rs`)

**Implement:**
- Generate unified diff between old and new file content (using `similar`)
- Parse diff for display: color added/removed lines
- Apply diff to file on disk
- Reverse diff (undo)

**Auto-test (unit):**
- Diff between two strings → correct unified diff
- Apply diff transforms source to target
- Reverse diff restores original
- Empty diff when files are identical

### M7.2 Edit confirmation flow (`src/agent/editor.rs`)

**Implement:**
- Generate file changes from core_model response
- Parse LLM output into structured `FileChange` (path, action, content)
- Present diff in chat panel
- Await user input: `y` approve, `n` reject, `e` edit manually
- On approval: write file, trigger syntax check
- On syntax failure: send to audit_model for fix

**Auto-test (unit with mocked pipeline):**
- Single file change → diff displayed → approve → file written
- Reject → no file written
- Syntax error after write → triggers audit fix cycle
- Multiple file changes presented sequentially

---

## Phase 8 — Wiring & End-to-End

### M8.1 End-to-end integration

**Implement:**
- Wire all modules in `main.rs`:
  1. Load/create config (M1.2)
  2. Create Ollama client (M1.3)
  3. Build AppState (M2.1)
  4. Initialize terminal (crossterm raw mode + alternate screen)
  5. Run event loop (M5.6)
  6. On input: route through agent pipeline (M3.1) based on mode
  7. Stream results to chat panel (M5.3)
  8. Handle file writes through sandbox (M4.1) and editor flow (M7.2)
  9. On quit: restore terminal, save session

**Auto-test (integration, `#[ignore]`):**
- Full cycle with live Ollama: ask to create a Python file → verify file exists
- Plan mode: verify no files created
- Auto mode: verify files created without confirmation
- Session restore: quit mid-conversation, restart, history preserved

### M8.2 Error handling & resilience

**Implement:**
- Ollama connection lost mid-session → show error, offer reconnect
- Model load failure (OOM) → suggest smaller model
- Terminal resize → re-render correctly
- Panic recovery: catch_unwind in main, restore terminal

**Auto-test (unit + integration):**
- Terminal resize from 120x40 to 80x24 → no crash, layout adapts
- Ollama disconnect during stream → error displayed, no crash
- Invalid UTF-8 in file → handled gracefully

---

## Phase 9 — Packaging & Distribution (v1.3)

### M9.1 Cross-platform builds

**Implement:**
- GitHub Actions matrix: linux (glibc + musl), macos (intel + arm), windows
- Static linking for musl target
- Strip binaries for size

**Auto-test (CI):**
- Build succeeds on all 5 targets
- Binary runs --version correctly on each

### M9.2 NPM wrapper

**Implement:**
- `npm/` directory with `package.json`: name `litecode-tui`, bin `litecode`
  pointing to a JS shim that finds and execs the platform-specific binary
- `install.js`: detect platform, extract correct binary from package
- `package.json` files array: include all platform binaries

**Auto-test:**
- `npm install -g .` succeeds
- `litecode --version` prints correct version
- Works on linux, macos, windows

---

## Test Infrastructure Summary

| Layer | Tool | Scope |
|-------|------|-------|
| Unit tests | `#[test]` + `wiremock` + `tempfile` | Each module in isolation |
| Property tests | `proptest` | Config parsing, diff, token math |
| Snapshot tests | `insta` | TUI rendering, diff display |
| Integration tests | `#[ignore]` + live Ollama | Full pipeline, file ops |
| CI | GitHub Actions | Build + test on 3 OS |
| Mutation tests | `cargo-mutants` | Test quality (optional, CI) |

### Test running commands

```bash
cargo test                          # All unit + snapshot tests
cargo test -- --ignored             # Integration tests (needs Ollama)
cargo test -p litecode-tui --test   # Specific integration test
cargo insta test                    # Snapshot test update
cargo insta review                  # Review snapshot diffs
cargo mutants                       # Mutation testing
```

---

## Implementation Order (Dependency Graph)

```
M0.1 ──→ M1.1 ──→ M1.2 ──→ M1.3 ──→ M1.4
                                    │
                    M2.1 ──→ M2.2 ←─┘
                      │
            M3.1 ←── M3.2
              │
         M3.3    M4.1 ──→ M4.2 ──→ M4.3
           │       │
           └───→ M7.1 ──→ M7.2
                      │
         M5.1 → M5.2 → M5.3 → M5.4 → M5.5 → M5.6
                                              │
                              M6.1    M6.2    │
                                │       │     │
                                └───→ M8.1 ──→ M8.2
                                                │
                                             M9.1 → M9.2
```

**Estimated milestone count:** ~20 milestones
**Estimated total test count:** ~150-200 tests across all layers
**v1.0 target:** M0.1 through M5.6 + M7.1 + M8.1 + M8.2
