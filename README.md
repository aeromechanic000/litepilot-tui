# LitePilot

Terminal AI coding assistant powered by local Ollama models. Written in Rust.

No API keys. No cloud. No data leaves your machine.

## Why LitePilot?

Every mainstream coding agent — Claude Code, Codex, Cursor, Copilot, Aider — sends your code to a cloud API. That means API costs per query, internet dependency, and your proprietary code on someone else's servers.

LitePilot runs entirely on your hardware using [Ollama](https://ollama.com)-host open-weight models. You buy the hardware once, and every query is free forever. Your code never leaves your machine.

This trade-off comes with real consequences: local 3–14B parameter models are less capable than Claude Opus or GPT-4. LitePilot compensates with a three-tier pipeline that routes tasks to the right model size, automatic response validation with retry, and a sandboxed auto-mode that plans, implements, and reviews in one pass.

### When to choose LitePilot

- You work on proprietary or sensitive code that cannot leave your machine
- You want zero per-query cost after hardware investment
- You work offline or in air-gapped environments
- You want full control over which model runs your code
- You prefer a terminal-native workflow over IDE integrations

### When to choose a cloud agent instead

- You need the best possible code quality (Claude Code, Codex)
- You want IDE-integrated autocomplete and inline suggestions (Copilot, Cursor)
- Your hardware cannot run models locally at acceptable speed
- You need multimodal input (images, screenshots → code)

## Comparison with Mainstream Coding Agents

### Feature Comparison

| Feature | LitePilot | Claude Code | Codex (OpenAI) | Cursor | Copilot | 
|---------|-----------|-------------|----------------|--------|---------|
| **Runtime** | Local (Ollama) | Cloud API | Cloud API | Cloud API | Cloud API | 
| **Privacy** | Full — no data leaves machine | Sends to Anthropic | Sends to OpenAI | Sends to OpenAI/other | Sends to GitHub/OpenAI | 
| **Cost** | Free (hardware only) | $0.01–0.10/query | $0.01–0.05/query | $20/mo or API costs | $10–19/mo | 
| **Offline** | Yes | No | No | No | No | No |
| **Interface** | Terminal (TUI) | Terminal (CLI) | Cloud IDE + CLI | IDE (VS Code fork) | IDE extension | 
| **Streaming** | Yes (SSE) | Yes | Yes | Yes | Yes | 
| **Multi-file edits** | Yes | Yes | Yes | Yes | Partial | 
| **Auto-apply** | Yes (Auto mode) | Yes | Yes | Yes | Yes (inline) | 
| **Confirmation flow** | Plan/Edit/Auto modes | Per-action | Per-task | Inline diff | Inline suggestion | 
| **Sandbox** | Path validation + cmd filter | Container | Sandbox | N/A | N/A | 
| **Web search** | Yes (DuckDuckGo) | Yes | Yes | Yes | Partial | 
| **Codebase context** | Built-in templates + RAG | CLAUDE.md + memory | Repo indexing | Repo indexing | Workspace indexing | 
| **Session persistence** | Yes | Yes | Yes | No | No | 
| **Custom skills/commands** | Yes (markdown) | Yes (hooks) | No | Rules files | No | 
| **Context window** | Model-dependent (2K–32K) | 200K | 128K+ | Model-dependent | Model-dependent | 
| **First-token latency** | 2–8s (local) | 1–3s (cloud) | 1–3s (cloud) | 1–3s (cloud) | <1s (cloud) | 
| **Code quality (complex tasks)** | Good | Excellent | Excellent | Excellent | Good | 
| **Code quality (simple tasks)** | Good | Excellent | Excellent | Excellent | Good | 
| **Large refactoring** | Moderate | Excellent | Good | Excellent | Moderate | 
| **Cross-file reasoning** | Limited | Excellent | Good | Good | Limited | 

### Advantages Over Cloud Agents

**Privacy and security.** Your code never touches a network. No API keys to manage, no Terms of Service governing your data, no risk of training on your proprietary code. For regulated industries (healthcare, finance, defense), this is the only option that guarantees data sovereignty.

**Zero marginal cost.** After the hardware investment, every query is free. No monthly subscriptions, no token billing surprises, no rate limiting. Run 1,000 queries or 1,000,000 — the cost is the same.

**Full offline operation.** Works on airplanes, in secure facilities, behind corporate firewalls, and in air-gapped networks. The only network dependency is the optional web search feature.

**Model sovereignty.** Choose exactly which model runs. Switch between Qwen, Gemma, Mistral, DeepSeek, Llama, or any Ollama-compatible model. Use different models for different tiers (fast planning, core coding, audit review). No vendor lock-in to a single provider.

**Transparent execution.** The three-tier pipeline is explicit: Plan (fast model) → Implement (core model) → Audit (audit model). You can see and control what each tier does. No black-box agent behavior.

**Sandboxed auto-mode.** Auto mode applies files without confirmation, but within strict sandbox constraints: path traversal blocking, symlink escape prevention, and command allowlisting. The safety net is enforced at the OS level, not by prompt engineering.

### Disadvantages vs Cloud Agents

**Model quality gap.** The best local models (Qwen3 14B, DeepSeek-Coder-V2 16B) are significantly less capable than Claude Opus/Sonnet or GPT-4 for complex tasks: large refactors, cross-file reasoning, architectural decisions, and nuanced bug analysis. This is the fundamental trade-off.

**Hardware requirements.** You need a machine capable of running models locally. For the recommended three-tier setup (4B + 8B + 14B), that means 16+ GB RAM and ideally a GPU. Performance scales directly with your hardware.

**Smaller context windows.** Local models typically support 2K–32K token contexts vs. 128K–200K for cloud models. LitePilot mitigates this with template-based RAG and token budgets, but it cannot match the raw context capacity of Claude Code or Codex.

**No multimodal input.** Cannot accept images, screenshots, or diagrams as input. Cloud agents like Claude Code can analyze UI screenshots and generate matching code.

**No IDE integration.** LitePilot is terminal-only. It does not provide inline autocomplete, ghost text suggestions, or direct editor integration like Copilot or Cursor.

**Slower first token.** Local inference on consumer hardware produces first tokens in 2–8 seconds vs. 1–3 seconds for cloud APIs. Subsequent token speed depends on your hardware but rarely matches cloud throughput.

**Less sophisticated tool use.** Cloud agents have structured tool APIs (file read/write, terminal, browser) with schema validation. LitePilot uses text-based file change markers, which are more fragile but more compatible with small local models.

### Where LitePilot Fits in a Developer Workflow

LitePilot is not a replacement for cloud agents — it is a complement:

- Use **Claude Code or Codex** for complex architecture decisions, large refactors, and tasks requiring top-tier reasoning
- Use **Copilot or Cursor** for inline autocomplete and IDE-integrated suggestions
- Use **LitePilot** for everyday tasks when privacy matters, when you are offline, or when you want free unlimited queries: file generation, boilerplate, simple bug fixes, code explanation, test generation, documentation

For solo developers and small teams on a budget, LitePilot alone can handle most daily coding tasks. For teams with compliance requirements, LitePilot is the agent you can deploy without legal review.

## Features

- **Local-first**: Uses Ollama-host models (3-5B fast, 6-7B core, 7-14B audit)
- **Three-tier pipeline**: Plan (fast model) → Implement (core model) → Audit (audit model)
- **Three modes**: Plan (read-only) → Edit (confirm each write) → Auto (sandboxed full-auto)
- **Skills system**: Built-in slash commands (`/search`, `/review`, `/explain`, `/simplify`, `/test`)
- **Auto-retry**: Validates model responses and retries with error context for small models
- **Syntax checking**: Multi-language syntax validation after file writes (Python, JS, Rust, Go, C/C++, Shell)
- **Streaming output**: Token-by-token rendering as the model generates
- **Web search**: Free DuckDuckGo search with disk-cached results for LLM context
- **Session persistence**: Save and resume conversations across sessions
- **Syntax highlighting**: Syntect-powered code block rendering in the terminal
- **Non-blocking UI**: Type and queue messages while the model is thinking
- **UV integration**: Manage Python projects with `/uv` commands
- **Cross-platform**: macOS (ARM64, x64) and Linux (ARM64, x64) via npm

## Quick Start

### 1. Install Ollama and pull a model

```bash
# Install Ollama (macOS / Linux)
curl -fsSL https://ollama.com/install.sh | sh

# Pull a model (recommended: qwen3.5:4b for fast responses on most hardware)
ollama pull qwen3.5:4b

# Or pull multiple for the three-tier pipeline
ollama pull qwen3:4b    # Fast  — planning, search
ollama pull qwen3:8b    # Core  — coding, generation
ollama pull qwen3:14b   # Audit — review and evaluate results
```

### 2. Install LitePilot

**Option A: Install from npm (recommended)**

```bash
npm install -g litepilot-tui
```

**Option B: Install from source**

```bash
# Requires Rust toolchain (https://rustup.rs)
git clone https://github.com/aeromechanic000/litepilot-tui.git
cd litepilot-tui
cargo install --path .
```

**Option C: Run without installing**

```bash
# Via npx (binary downloaded on first run)
npx litepilot-tui

# Or from source
git clone https://github.com/aeromechanic000/litepilot-tui.git
cd litepilot-tui
cargo run --release
```

### 3. Launch

```bash
# Start Ollama first
ollama serve

# Run LitePilot in your project directory
cd ~/my-project
litepilot

# Or specify a directory
litepilot -d /path/to/project
```

On first launch, a setup wizard helps you configure:
1. Ollama endpoint URL (default: `http://127.0.0.1:11434`)
2. Model selection for Fast, Core, and Audit slots
3. Confirmation and save

## Typical Usage

### Ask questions about your code

```
What does the handle_input function do in src/main.rs?
```

LitePilot reads your project files and answers in context.

### Generate code

```
Create a Python REST API with Flask for a todo list with CRUD endpoints
```

The response includes file changes with `### FILE:` and `### ACTION:` markers. Type `/apply` to write the files to disk.

### Fix bugs

```
Fix the divide function in bug.py to handle division by zero
```

LitePilot shows the proposed fix. Review it, then `/apply` to write.

### Add features to existing code

```
Add a Circle class and a Triangle class to shapes.py, each with an area() method
```

### Refactor code

```
Refactor main.rs to extract the event handling into a separate function
```

### Generate tests

```
Write unit tests for the Sandbox::validate_path function in src/sandbox/mod.rs
```

### Use skills for focused tasks

```
/explain what does the fn main() function do in a Rust program
/review src/main.rs
/search find all TODO comments in the project
/simplify make the handle_input function shorter
/test generate tests for src/util/text.rs
```

### Queue messages while processing

While the model is thinking, type and send more messages. They appear with a `>` prefix and are processed in order after the current response completes.

## Key Bindings

| Key | Action |
|-----|--------|
| `Enter` | Send input |
| `Shift+Tab` | Switch mode (Plan → Edit → Auto) |
| `Ctrl+S` | Toggle web search |
| `Ctrl+C` | Quit (double-press in Auto mode) |
| `Esc` | Toggle sidebar |
| `PageUp` / `PageDown` | Scroll chat history |
| `Tab` | Switch sidebar tab (when sidebar visible) |
| `Up` / `Down` | Navigate sidebar |

## Slash Commands

| Command | Description |
|---------|-------------|
| `/skills` | List all available skills |
| `/setup` | Re-run the setup wizard |
| `/apply` | Write file changes from the last response to disk |
| `/search <query>` | Search local files using grep/find |
| `/review <path>` | Review code for bugs, style, and security |
| `/explain <question>` | Explain code in plain language |
| `/simplify <instruction>` | Simplify and refactor code |
| `/test <path>` | Generate comprehensive tests |
| `/uv init` | Initialize a Python project with uv |
| `/uv venv` | Create a virtual environment |
| `/uv add <package>` | Add a Python dependency |
| `/uv run <script>` | Run a Python script |
| `/quit` or `/exit` | Exit LitePilot |

## Configuration

Config lives at `~/.litepilot/config.toml`. The setup wizard creates it on first run.

```toml
ollama_endpoint = "http://127.0.0.1:11434"
fast_model = "qwen3:4b"
core_model = "qwen3:8b"
audit_model = "qwen3:14b"
default_mode = "edit"
max_retries = 3
connect_timeout = "10s"
```

### Project-Local Config

Place a `.litepilot/config.toml` in your project root to override global settings for that project.

### Custom Skills

Create a markdown file in `~/.litepilot/skills/` with YAML frontmatter:

```markdown
---
name: my-skill
description: What this skill does
trigger: keyword1, keyword2
---

Your skill prompt goes here.
This guides the model when /my-skill is invoked.
```

## Three Modes

| Mode | Description | File Writes |
|------|-------------|-------------|
| **Plan** | Read-only analysis, architecture suggestions | Blocked |
| **Edit** | Generate code with diff preview, confirm each write | Requires `/apply` or `y/n` |
| **Auto** | Full pipeline: plan → implement → audit → auto-apply | Automatic (sandboxed) |

Toggle with `Shift+Tab`.

## Recommended Models

| Tier | Size | Purpose | Examples |
|------|------|---------|----------|
| Fast | 3-5B | Easy/quick tasks, simple questions, search | `qwen3.5:4b`, `gemma3:4b` |
| Core | 6-7B | Main coding, implementation, file generation | `qwen3:8b`, `mistral:7b` |
| Audit | 7-14B | Review and evaluate results against requirements | `qwen3:14b`, `deepseek-coder-v2:16b` |

A single model works for all tiers. The three-tier setup gives better results on larger hardware.

## Architecture

```
src/
├── main.rs              Entry point, CLI args, event loop
├── app.rs               Application state machine (AppState, modes)
├── config.rs            config.toml parsing, defaults, setup wizard
├── wizard.rs            First-run setup wizard (TUI)
├── skills/              Skills system (~/.litepilot/skills/)
├── agent/               Agent orchestrator, retry, prompts, syntax
├── ollama/              Ollama client (connectivity, chat, model info)
├── sandbox/             Path validation, command allowlist
├── search/              Free web search + local cache
├── project/             File tree, git status, file ops, UV integration
├── codebase/            Built-in code templates + tag-based RAG
├── session/             Conversation persistence
├── ui/                  TUI rendering (ratatui + crossterm)
└── util/                Diff generation, text utilities
```

## License

MIT
