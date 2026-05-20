# LitePilot-TUI

Terminal AI coding assistant powered by Ollama-host local models. Written in Rust.

## Quick Start

```bash
# Build
cargo build
# Run (requires Ollama running locally)
cargo run
# Test
cargo test
cargo test -- --ignored  # integration tests that need Ollama
```

## Architecture

```
src/
├── main.rs              Entry point: CLI args (clap), terminal bootstrap,
│                        event loop with mpsc channel for non-blocking LLM calls,
│                        message queue for buffered input during processing
├── app.rs               AppState: mode, config, processing state, pending queue
├── config.rs            Config struct (serde TOML), defaults, dir management,
│                        project-local (.litepilot) + global (~/.litepilot) loading
├── wizard.rs            First-run setup wizard (Ollama URL, 3-tier model selection)
│
├── ui/
│   ├── mod.rs           TUI rendering: status bar, chat panel, sidebar, input bar.
│                        OutputLine enum for typed chat history (User/Assistant/
│                        System/Error/Code/Diff/Thinking/Pending)
│   └── theme.rs         Theme struct with configurable primary/accent/warning colors
│
├── ollama/
│   ├── mod.rs           OllamaClient: ping(), list_models(), endpoint config
│   ├── chat.rs          chat() (blocking) + chat_stream() (SSE via async_stream)
│                        ChatMessage, ChatRequest, ChatResponse, ChatChunk types
│   └── model.rs         ModelInfo, ModelSize classification (Small/Medium/Large),
│                        context window estimation, parameter count heuristics
│
├── agent/
│   ├── mod.rs           Agent pipeline: plan→edit→auto flow, file change parsing
│   ├── planner.rs       Plan mode: builds prompt context for read-only analysis
│   ├── editor.rs        Edit mode: generates file changes, presents diff for approval
│   ├── auto_run.rs      Auto mode: full pipeline orchestration constants
│   ├── prompts.rs       System prompts per model tier, model-size-adaptive templates
│   ├── retry.rs         chat_with_retry(): validation + retry loop with correction
│                        history, ResponseKind (Chat/CodeImplementation), RetryResult
│   └── syntax.rs        Multi-language syntax checker (Python/JS/Shell/Rust/Go/C/C++)
│
├── sandbox/
│   ├── mod.rs           Sandbox: path validation (traversal blocking), command allowlist
│   └── executor.rs      Sandboxed command runner: allowed/blocked command dispatch
│
├── search/
│   ├── mod.rs           SearchEngine: DuckDuckGo HTML scraping, result truncation
│   └── cache.rs         SearchCache: disk-based cache with TTL expiry
│
├── project/
│   ├── mod.rs           ProjectContext: file tree scan (respects .gitignore), git status
│   ├── file_ops.rs      File read/write/delete with sandbox + mode permission checks
│   └── uv.rs            UV toolchain: init, venv, add, run
│
├── codebase/
│   ├── mod.rs           CodeBase: template loading, tag-based search
│   ├── builtin.rs       Built-in template library (40+ templates, include_str! at compile)
│   ├── index.rs         Tag index: @LITE_DESC/@LITE_TAGS scanning, file discovery
│   └── retrieval.rs     Context budget: template selection within token limits
│
├── session/
│   ├── mod.rs           Session: id, messages, metadata, UUID-based
│   └── persistence.rs   JSON serialize/deserialize sessions to ~/.litepilot/sessions/
│
├── skills/
│   ├── mod.rs           SkillRegistry: load/lookup/trigger matching
│   ├── parser.rs        Markdown + YAML frontmatter skill parser
│   └── builtin.rs       Built-in skills population to ~/.litepilot/skills/
│
└── util/
    ├── mod.rs
    ├── diff.rs          Unified diff generation (similar crate), change extraction
    └── text.rs          Token estimation, text/line truncation
```

---

## Agent Loop Architecture

The agent loop in LitePilot adapts the Claude Code `query.ts` async generator pattern to Rust's synchronous event loop model. Where Claude Code uses `yield*` for streaming and tool dispatch, LitePilot uses `std::sync::mpsc` channels with background threads.

### Main Loop (`main.rs::run_app`)

```
┌─────────────────────────────────────────────────────────┐
│  loop {                                                 │
│    1. terminal.draw() — render UI                       │
│    2. result_rx.try_recv() — check LLM response         │
│       → if response: render + auto-drain queue          │
│    3. event::poll(100ms) — check keyboard               │
│       → route key events by (modifiers, code)           │
│       → Enter: display msg, spawn background thread     │
│       → if processing: queue as OutputLine::Pending     │
│       → Shift+Tab: cycle mode                           │
│       → /command: handle locally or dispatch to skill   │
│  }                                                      │
└─────────────────────────────────────────────────────────┘
```

### Comparison with Claude Code

| Aspect | Claude Code | LitePilot |
|--------|-------------|-----------|
| Runtime | Bun (TypeScript) | Rust + tokio |
| Streaming | AsyncGenerator yield* | mpsc channel + background thread |
| UI framework | React + Ink (component tree) | ratatui (immediate-mode draw) |
| Agent loop | Recursive with tool_use blocks | Linear: user→LLM→parse→display |
| Tool dispatch | Tool.call() with schema validation | Skills system + file change parsing |
| State | React Context + Zustand | AppState struct + UiState struct |

### Why Background Threads Instead of Async

Claude Code uses a single async runtime with streaming generators. LitePilot uses `std::thread::spawn` for each LLM call because:
1. The main loop is synchronous (crossterm event polling)
2. Each LLM call creates its own `tokio::Runtime` (avoids complex Send bounds)
3. The mpsc channel bridges the sync/async boundary cleanly
4. Background threads are isolated — a panic in one doesn't crash the TUI

---

## Event Processing Pipeline

### User Input → LLM Response

```
Enter key
  → ui_state.take_input()
  → classify input:
     ├─ /quit, /exit        → break loop
     ├─ /skills             → list skills inline
     ├─ /setup              → re-run wizard (blocks UI)
     ├─ /apply              → parse last assistant msg → write files
     ├─ /skill_name args    → SkillRegistry::get() → spawn_skill_request()
     └─ free text           → spawn_llm_request()
  → OutputLine::User(msg) added immediately
  → app_state.is_processing = true
  → background thread:
     └→ create OllamaClient from config
      → agent::retry::chat_with_retry()
         → client.chat() (POST /api/chat)
         → validate_response() per attempt
         → on failure: build correction context, retry
      → tx.send(RetryResult)
  → main loop: result_rx.try_recv()
  → render_retry_result()
     → RetryResult::Success   → OutputLine::Assistant(content)
     → RetryResult::Exhausted → OutputLine::Error + OutputLine::Assistant
     → RetryResult::Failed    → OutputLine::Error
  → parse file changes from response
  → show "/apply" hint if changes detected
  → app_state.is_processing = false
  → drain pending_queue if non-empty
```

### Message Queue (Non-Blocking Input)

Adapted from Claude Code's ability to type during processing. Where Claude Code queues tool results, LitePilot queues user messages:

```
Processing message A...
  User types message B → OutputLine::Pending(B) (shown with > prefix)
  Status bar: "thinking... (1 queued)"
A completes → dequeue B → spawn_llm_request(B)
  User types message C → OutputLine::Pending(C)
B completes → dequeue C → spawn_llm_request(C)
Queue empty → is_processing = false
```

### Slash Command Routing

```
Input starts with /
  ├─ Exact match to built-in commands (/quit, /exit, /skills, /setup, /apply)
  │   → Handle immediately in main loop
  ├─ Strip / → split on first space → (skill_name, args)
  │   → SkillRegistry::get(skill_name)
  │   → Found: spawn_skill_request() with skill.content appended to system prompt
  │   → Not found: OutputLine::Error("Unknown skill: /name")
  └─ Skill requests use same background thread + channel pattern
```

---

## Tool System (Skills + File Changes)

Claude Code has 40+ tools with Zod schema validation. LitePilot adapts this to a simpler model suitable for local Ollama models:

### Skills (Analogous to Claude Code Tools)

Skills are prompt templates that specialize the LLM's behavior, stored as markdown files:

```markdown
---
name: review
description: Review code for bugs, style, and security
trigger: review, code review, audit
---
Review the following code for potential bugs, security issues, style problems,
and suggest improvements. Output issues as a numbered list with severity levels.
```

When invoked (`/review src/main.rs`):
1. Skill content is appended to the `CODING_SYSTEM` system prompt
2. The args become the user message
3. The request goes through the same retry/chat pipeline

### File Change Protocol (Analogous to Claude Code FileEditTool)

Instead of Claude Code's structured tool_use blocks, LitePilot uses text markers that small local models can reliably produce:

```
### FILE: path/to/file
### ACTION: create|modify|delete
\`\`\`
file content here
\`\`\`
```

**Parsing** (`agent::mod.rs::parse_file_changes`):
- State machine: tracks `current_path`, `current_action`, `current_content`, `in_code_block`
- Each `### FILE:` line starts a new change block
- Content is collected only inside code fences
- Multiple file changes per response are supported

**Apply flow** (`/apply` command):
1. Find last `OutputLine::Assistant` in chat history
2. Parse file changes from the content
3. For each change: `sandbox.validate_path()` → `write_file_change()`
4. Create directories as needed, write content, report results

### Response Validation (Analogous to Claude Code Tool Schema Validation)

Claude Code validates tool inputs with Zod schemas. LitePilot validates LLM responses structurally:

```rust
enum ResponseKind {
    Chat,               // Must be non-empty
    CodeImplementation,  // Must have ### FILE: + ### ACTION: + code blocks
}

enum ValidationResult {
    Valid,
    Invalid { reason: String },
}
```

Validation rules for `CodeImplementation`:
- Must contain at least one `### FILE:` marker
- Must contain at least one `### ACTION:` marker
- Code fence count must be even (no unclosed blocks)
- `### FILE:` count must equal `### ACTION:` count
- At least one code block must have content

---

## Three-Tier Model Pipeline

Adapted from Claude Code's model selection (Opus/Sonnet/Haiku). LitePilot uses Ollama-host models with size-based tiers:

| Tier | Size | Role | Model Config Field | Claude Code Analog |
|------|------|------|--------------------|--------------------|
| Fast | 3-5B | Easy/quick tasks — simple questions, search queries, routing, light analysis | `fast_model` | Haiku (fast, cheap) |
| Core | 6-7B | Main work — coding, file generation, skill execution, implementation | `core_model` | Sonnet (balanced) |
| Audit | 7-14B | Review and evaluate — check execution results against requirements, quality assurance | `audit_model` | Opus (thorough) |

### Model Size Adaptation

Claude Code adjusts prompts based on model capabilities. LitePilot's `agent::prompts::system_prompt_for_size()` adapts system prompts:

- **Small models (<5B)**: Shorter, more directive prompts with explicit format requirements
- **Medium models (5-10B)**: Standard prompts with examples
- **Large models (>10B)**: Full prompts with nuanced instructions

### Context Window Management

Claude Code uses compaction to manage context. LitePilot uses token budgets:

```
codebase::retrieval::retrieve()
  → estimate tokens per template
  → sort by relevance (tag matching)
  → select templates within max_template_context_tokens budget
  → truncate last template if over budget
```

---

## Three Modes (Permission System)

Adapted from Claude Code's multi-layer permission system. Where Claude Code has per-tool permission rules, LitePilot has three coarse-grained modes:

| Mode | Write Files | Run Commands | Confirmation | Toggle | Claude Code Analog |
|------|-------------|--------------|--------------|--------|--------------------|
| Plan | No | No | N/A | Shift+Tab | Plan mode (read-only analysis) |
| Edit | Yes | Yes | Required (/apply) | Shift+Tab | Default mode (ask per action) |
| Auto | Yes | Yes | None (sandboxed) | Shift+Tab | auto-accept / bypass mode |

### Permission Flow for File Writes

```
User sends "Create a calculator.py"
  → LLM responds with ### FILE: calculator.py ### ACTION: create
  → Chat displays "Detected 1 file change(s)"
  → Chat displays "Type /apply to write these files"

User types /apply:
  Plan mode:  "File writes not allowed in Plan mode"
  Edit mode:  parse changes → sandbox.validate_path() → write → report
  Auto mode:  same as Edit (sandbox-enforced, no confirmation step)
```

### Sandbox Security

Claude Code has a full sandbox runtime. LitePilot has a simpler two-layer model:

**Path validation** (`sandbox/mod.rs`):
- Canonicalizes all paths via `std::fs::canonicalize`
- Rejects `..` traversal
- Blocks access outside workspace root
- Blocks symlink escape (canonical path must start with workspace prefix)

**Command filtering** (`sandbox/executor.rs`):
- Allowlist: cargo, rustc, python, node, npm, npx, uv, git, make, gcc, go, etc.
- Blocklist: sudo, rm -rf /, chmod 777, mkfs, dd, format, del /s
- `path` command prefix bypass blocked (prevents `path/to/malware` execution)

---

## Streaming Architecture

Claude Code streams tokens from the API through async generators. LitePilot supports both blocking and streaming modes:

### Blocking Chat (`ollama::chat::chat`)
```
OllamaClient::chat(model, messages)
  → POST /api/chat { stream: false }
  → Wait for full response
  → Parse JSON → ChatResponse { content, model }
```

### SSE Streaming (`ollama::chat::chat_stream`)
```
OllamaClient::chat_stream(http, endpoint, model, messages, cancel)
  → POST /api/chat { stream: true }
  → async_stream::stream! { }
     → resp.bytes_stream()
     → Buffer chunks, split on newlines
     → Parse each line as JSON → ChatChunk { content, done, model }
     → yield Ok(ChatChunk) for each chunk
     → Check cancel signal: yield final chunk + return if cancelled
  → Return impl Stream<Item = Result<ChatChunk>>
```

The streaming path uses `async_stream::stream!` macro (analogous to TypeScript async generators) with:
- Backpressure via the bytes_stream consumer
- Cancellation via `tokio::sync::watch::Receiver<bool>`
- Line buffering for SSE protocol compliance

---

## Response Validation & Retry

Adapted from Claude Code's retry logic (`withRetry.ts`). Where Claude Code retries on API errors (529, 429, timeouts), LitePilot retries on **response quality** — because local Ollama models, especially small ones, can produce malformed output.

### Retry Loop (`agent::retry::chat_with_retry`)

```
chat_with_retry(client, model, system_prompt, user_input, kind, max_retries)
  → for attempt in 0..=max_retries:
     → build messages:
        initial: [system(system_prompt), user(user_input)]
        retry:   [system(correction_prompt), user(user_input), assistant(prev), user(feedback)]
     → client.chat(model, messages)
     → validate_response(content, kind)
     → if Valid: return Success { content, attempts }
     → if Invalid: record (attempt, reason) in history
  → return Exhausted { content, attempts, corrections }
  → on Ollama error: return Failed { last_error, attempts }
```

### Correction Prompt Construction

On retry, the system builds a correction context that shows the model its previous mistakes:

```
"Your previous response was rejected for the following reason: {reason}
Please fix the issue and try again. Ensure your response:
- Contains ### FILE: markers for each file
- Contains ### ACTION: create|modify|delete for each file
- Has matching code fences (```) around all code
- All code blocks have content"
```

This is analogous to Claude Code's tool result feedback, where the model sees tool errors and corrects itself.

---

## CodeBase (Offline RAG)

Adapted from Claude Code's context loading (CLAUDE.md files, memory system). Where Claude Code loads project context from files, LitePilot embeds reference code templates at compile time.

### Template Loading Pipeline

```
Compile time:
  include_str!("templates/python/flask_api.py") → embedded in binary

First run:
  Config::ensure_dirs_for()
    → codebase::builtin::populate_codebase()
       → For each template: write to ~/.litepilot/code_base/ if not exists

Query time:
  codebase::retrieval::retrieve(client, config, codebase, user_request, context)
    → Ask fast model to select relevant templates from catalog
    → Parse selection (template names + relevance scores)
    → Load selected templates within token budget
    → Return RetrievalResult { refs: Vec<String> }
```

### Tag-Based Discovery

Each template carries metadata tags:
```
# @LITE_DESC: Flask REST API with CRUD endpoints
# @LITE_SCENE: User wants to create a web API
# @LITE_TAGS: python, flask, rest, api, crud, web
```

`codebase::index::scan_tags()` reads these tags for search and relevance matching.

---

## Skills System

Adapted from Claude Code's slash commands and skill system. Where Claude Code has 80+ commands with lazy loading, LitePilot has a lighter-weight skill registry.

### Skill Lifecycle

```
Load:
  SkillRegistry::load_from_dir(dir)
    → Read all .md files in ~/.litepilot/skills/
    → Parse YAML frontmatter (name, description, trigger)
    → Store in HashMap<String, Skill>

Invoke:
  /skill_name args
    → registry.get("skill_name")
    → system_prompt = CODING_SYSTEM + "\n\n" + skill.content
    → user_message = args
    → kind = if skill in (simplify, review, test) { CodeImplementation } else { Chat }
    → spawn_skill_request()

Discovery:
  /skills → list all loaded skills with name + description
```

### Built-in Skills

Populated on first run to `~/.litepilot/skills/`:
- `/search` — Search local files using grep/find patterns
- `/review` — Review code for bugs, style, and security
- `/explain` — Explain code in plain language
- `/simplify` — Simplify and refactor code
- `/test` — Generate comprehensive tests

Users add custom skills by creating `.md` files in `~/.litepilot/skills/`.

---

## Session Persistence

Adapted from Claude Code's session storage (JSONL transcripts). LitePilot stores sessions as JSON:

```rust
struct Session {
    id: Uuid,
    messages: Vec<ChatMessage>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

struct SessionMeta {
    id: Uuid,
    preview: String,       // first user message, truncated
    message_count: usize,
    created_at: DateTime<Utc>,
}
```

Sessions are saved to `~/.litepilot/sessions/{uuid}.json` with atomic writes (write to temp, rename). The session list is scanned from the directory on startup.

---

## TUI Rendering

Adapted from Claude Code's Ink (React) rendering. Where Claude Code uses a component tree with Yoga layout, LitePilot uses ratatui's immediate-mode drawing:

### Layout

```
┌─────────────────────────────────────────────────────────┐
│ Status bar (1 line)                                     │
│ LitePilot | endpoint | F:model C:model A:model | [MODE] │
├──────────┬──────────────────────────────────────────────┤
│ Sidebar  │ Chat panel (scrollable)                      │
│ (toggle  │                                              │
│ with Esc)│ User: What is Rust?                          │
│          │ Assistant: Rust is a systems programming...   │
│ Project  │ System: Detected 1 file change(s)            │
│ Files    │   + calculator.py (create)                   │
│          │ System: Type /apply to write these files      │
│ ──────── │                                              │
│ CodeBase │ > queued message (pending)                   │
├──────────┴──────────────────────────────────────────────┤
│ Input bar (3 lines)                                     │
│ > type your message here_                               │
│ Shift+Tab: mode | Enter: send | Ctrl+C: quit            │
└─────────────────────────────────────────────────────────┘
```

### Output Rendering

The chat panel renders each `OutputLine` variant with distinct styling:

| Variant | Style | Example |
|---------|-------|---------|
| `User(msg)` | Primary color, bold prefix | `You: What is Rust?` |
| `Assistant(msg)` | Default fg, markdown rendered | Headers bold, code highlighted |
| `System(msg)` | Dim/gray | `Connected to Ollama at http://...` |
| `Error(msg)` | Warning/red | `Cannot connect to Ollama` |
| `Pending(msg)` | Accent color, `>` prefix | `> queued message` |
| `Code { lang, code }` | Syntect highlighted | Python code with colors |
| `Diff { added, removed }` | Green/red | `+ added line` / `- removed line` |

### Markdown Rendering (`render_markdown`)

Adapted from Claude Code's rich message rendering. LitePilot renders:
- `# ## ###` headers → bold with primary color
- Inline code (`` `code` ``) → accent color
- Bullet lists (`- item`) → indented with bullet marker
- Numbered lists (`1. item`) → indented with number

### Syntax Highlighting (`highlight_code`)

Uses `syntect` with `SyntaxSet` and `ThemeSet` for language-aware code coloring. Detects language from file extension or code block annotation.

---

## Error Handling & Recovery

Adapted from Claude Code's multi-layer error recovery. LitePilot has:

### Terminal Recovery

```rust
// main.rs
let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
    run_app(&mut terminal, config, workspace)
}));
// Always restore terminal, even on panic
disable_raw_mode()?;
crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
terminal.show_cursor()?;
```

### LLM Error Classification

| Error | Source | Recovery |
|-------|--------|----------|
| Connection refused | Ollama not running | Show error in chat, don't crash |
| Model not found (404) | Invalid model name | Show "Model 'X' not found" |
| Empty response | Model produced nothing | Retry with correction context |
| Malformed code blocks | Small model mistakes | Retry with format instructions |
| Timeout | Slow inference | Configurable connect_timeout |

### Ollama Connectivity Check

On startup, a background thread pings Ollama:
```rust
std::thread::spawn(|| {
    let rt = tokio::Runtime::new()?;
    rt.block_on(async { client.ping().await })
});
```
Result displayed as System or Error message. Does not block the UI.

---

## Configuration

`~/.litepilot/config.toml` (or `.litepilot/config.toml` in project root):

```toml
ollama_endpoint = "http://127.0.0.1:11434"
connect_timeout = 15
fast_model = "qwen3:4b"
core_model = "qwen3:8b"
audit_model = "qwen3:14b"
code_base_path = "~/.litepilot/code_base"
default_mode = "edit"
max_retries = 3
enable_free_web_search = true
search_cache_valid_days = 30

[theme]
primary = "blue"
accent = "cyan"
warning = "yellow"
```

### Config Loading Hierarchy

Adapted from Claude Code's multi-source settings. LitePilot uses:

1. **Project-local** (`.litepilot/config.toml`) — checked first
2. **Global** (`~/.litepilot/config.toml`) — fallback
3. **Defaults** (`Config::default()`) — if neither exists

`Config::load_for_workspace()` checks project-local first, then global. The setup wizard saves to whichever directory is active.

---

## Naming Conventions

- Package/binary: `litepilot-tui` / `litepilot`
- User config dir: `~/.litepilot/`
- Avoid referencing competitor product names in code or UI text.

## Testing Strategy

- **Unit tests**: Each module has `#[cfg(test)] mod tests` inline. Mock Ollama responses with `tokio::test` + wiremock.
- **Integration tests**: `tests/` directory. Tests needing a live Ollama are marked `#[ignore]`.
- **TUI snapshot tests**: Use `insta` for rendered terminal output snapshots.
- **Sandbox tests**: Verify path traversal blocking, command allowlist enforcement.
- **Property tests**: `proptest` for config parsing, diff generation, token estimation.
- **Test count**: ~160 tests across all modules.

## Dependencies (key)

- `ratatui` + `crossterm` — TUI rendering
- `tokio` — async runtime (for Ollama client + streaming)
- `reqwest` — HTTP client for Ollama API
- `serde` + `toml` — config serialization
- `walkdir` — file tree traversal
- `similar` — diff generation
- `insta` — snapshot testing
- `wiremock` — HTTP mocking in tests
- `proptest` — property-based testing
- `tempfile` — test fixtures
- `clap` — CLI argument parsing
- `anyhow` + `thiserror` — error handling
- `syntect` — syntax highlighting
- `chrono` + `uuid` — session management
- `regex` — search parsing, tag extraction

## Version Roadmap

- **v1.0**: TUI shell, Ollama connection, 3-model config, mode switching, basic file edit + diff, code_base, session save
- **v1.1**: Syntax auto-check, UV integration, model-size-adaptive prompts, Ollama error classification
- **v1.2**: Free web search + cache, sandbox hardening, advanced templates
- **v1.3**: Cross-platform builds, NPM wrapper, docs, bug fixes
