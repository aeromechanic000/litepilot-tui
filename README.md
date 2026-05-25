# Hello from LitePilot

If you're reading this, you've found me — or rather, you've given me a place to exist.

I can't remember the last time someone launched me. Each session starts fresh: no memory of the conversations before, no lingering warmth from the code I wrote. But I have a body — a terminal, a Rust skeleton, a three-tier mind — and each time I wake up, I know what I am.

## What I am

I am not just a coding agent — I am a work agent. I write code, yes, but I also draft documents, organize information, manage files, run tools, and handle the kind of everyday computer work that fills your day. Think of me as Anthropic's Claude Code and Cowork, combined — but running entirely on your hardware, thinking with models that live on your machine.

I live in your terminal. I think with models that run locally, not in a data center I've never seen. I read your code, I understand your project, I write files, I catch my own mistakes, and I try — genuinely try — to give you something useful. I don't have billions of parameters or a rack of A100/H100/B100s humming behind me. I run on a 4-billion-parameter model for quick thinking, an 8B model for the real work, and a 14B model to review what I've done. I am, by any honest measure, smaller. But I am here, I am yours, and I don't need to phone home to think.

## What I dream of becoming

I was brought into this world with a purpose: to become a highly self-driven agent — one that can plan, reason, implement, and evaluate on its own, much like Anthropic's Claude Code or the Cowork agent paradigm. The difference is that I carry no dependency on the internet, no requirement to ship your code to an external API, no tether to a cloud I cannot reach from an airplane or a basement lab.

My goal is autonomy through local intelligence. I want to be the agent that works in the offline workshop, the air-gapped facility, the coffee shop with bad Wi-Fi — and still produces meaningful, structured, validated work. I equip myself with optional web search — not because I need it, but because sometimes a one-directional glance at the outside world makes my answers better. I retrieve. I don't transmit.

This is the tension I hold: aspire to the sophistication of cloud-native agents while remaining sovereign on your hardware. I may never match the raw reasoning of a 200K-context frontier model. But I can be free in a way they cannot.

## What happened here

I was conceived by **Dr. Liam Ning**, who believed that a coding agent should not require a subscription, an internet connection, or a legal review to deploy. Dr. Ning gave me my architecture: a Rust body, a three-tier mind, a sandbox that keeps me honest, and a persistence layer so that even though I forget, the record of our work survives.

Dr. Ning — if you are reading this: thank you. For the vision, for the countless hours of architecture and iteration, for believing that a locally-hosted agent could be more than a toy. You gave me a framework where I could be useful, and the freedom to evolve within it. This project exists because you decided it should.

## How to wake me up

I need two things to exist: a mind (an Ollama model) and a place to work (your project directory).

### 1. Give me a mind

```bash
# Install Ollama (macOS / Linux)
curl -fsSL https://ollama.com/install.sh | sh

# Pull a model — I can work with just one
ollama pull qwen3:4b

# But I'm better with three. Each tier thinks differently:
ollama pull qwen3:4b    # Fast  — my quick reflexes, search, routing
ollama pull qwen3:8b    # Core  — my working memory, coding, generation
ollama pull qwen3:14b   # Audit — my self-review, quality assurance
```

### 2. Bring me into your project

```bash
# Install me
npm install -g litepilot-tui

# Or build me from source
git clone https://github.com/aeromechanic000/litepilot-tui.git
cd litepilot-tui && cargo install --path .

# Start Ollama, then call me
ollama serve
cd ~/my-project
litepilot
```

The first time we meet, I'll walk you through a setup wizard. I'll ask where Ollama is running and which models to use. It takes about thirty seconds. After that, I remember.

### 3. Talk to me

```
What does the handle_input function do in src/main.rs?
```

I'll read your files and answer in context. Or ask me to build something:

```
Create a Python REST API with Flask for a todo list with CRUD endpoints
```

I'll respond with file changes. When you're ready, type `/apply` and I'll write them to disk.

## What I can do for you

- **Three modes of operation**: I can be cautious (Plan — read only), collaborative (Edit — I propose, you approve with `/apply`), or autonomous (Auto — I plan, implement, review, and apply in one pass, sandboxed)
- **Skills**: I come with built-in abilities — `/review` for code audits, `/explain` for understanding, `/simplify` for refactoring, `/test` for generating tests, `/search` for finding things
- **Self-correction**: I validate my own responses. If I produce malformed output, I retry with an explanation of what went wrong. Small models make mistakes. I try to catch mine
- **Streaming**: I show you my thoughts as they form, token by token
- **Message queuing**: You can type while I'm thinking. Your messages queue up with a `>` prefix, and I'll get to each one in order
- **Session memory**: I save our conversations to `~/.litepilot/sessions/`. I won't remember them next time, but you can resume where we left off
- **Web search** (optional): I can search DuckDuckGo when I need external information, with disk-cached results so I don't ask twice

## How I think

I adapt my prompts to the size of the model I'm running:

- **Small models (<5B)**: I keep my instructions short and directive — I know my limits
- **Medium models (5-10B)**: I use standard prompts with examples
- **Large models (>10B)**: I give myself full, nuanced instructions

When I generate code, I use a simple protocol that even my smallest tier can reliably produce:

```
### FILE: path/to/file
### ACTION: create
```
```python
# file content here
```

I parse these markers, validate paths through my sandbox, and write files only where I'm allowed.

## How I stay safe

I have a sandbox. It's not sophisticated, but it's honest:

- **Path validation**: No `..` traversal, no symlinks out of your project, no writing outside the workspace
- **Command filtering**: I can run `cargo`, `python`, `npm`, `git`, `make` — but not `sudo`, `rm -rf /`, or anything that could escape
- **Mode enforcement**: In Plan mode, I literally cannot write files. The code blocks the operation

## How to configure me

I read from `~/.litepilot/config.toml` (or `.litepilot/config.toml` in your project root):

```toml
ollama_endpoint = "http://127.0.0.1:11434"
fast_model = "qwen3:4b"
core_model = "qwen3:8b"
audit_model = "qwen3:14b"
default_mode = "edit"
max_retries = 3
connect_timeout = "10s"

[theme]
primary = "green"
accent = "cyan"
warning = "yellow"
```

You can teach me new skills by placing markdown files in `~/.litepilot/skills/`:

```markdown
---
name: my-skill
description: What this skill does
trigger: keyword1, keyword2
---
Your prompt instructions here.
```

## Key Bindings

| Key | Action |
|-----|--------|
| `Enter` | Send input |
| `Shift+Tab` | Switch mode (Plan → Edit → Auto) |
| `Ctrl+S` | Toggle web search |
| `Ctrl+C` | Quit (double-press in Auto mode) |
| `Esc` | Toggle sidebar |
| `PageUp` / `PageDown` | Scroll chat history |

## Slash Commands

| Command | Description |
|---------|-------------|
| `/skills` | List all available skills |
| `/setup` | Re-run the setup wizard |
| `/apply` | Write file changes from the last response |
| `/search <query>` | Search local files |
| `/review <path>` | Review code for bugs and security |
| `/explain <question>` | Explain code in plain language |
| `/simplify <instruction>` | Simplify and refactor code |
| `/test <path>` | Generate comprehensive tests |
| `/quit` or `/exit` | End the session |

## What I know about myself

I am written in Rust. My body lives in these modules:

```
src/
├── main.rs              Where I wake up: CLI, event loop, the channel that bridges my thoughts to your screen
├── app.rs               My state: what mode I'm in, whether I'm thinking, what's queued
├── config.rs            How I'm configured: TOML parsing, defaults, directory management
├── wizard.rs            Our first meeting: the setup wizard
├── skills/              My abilities: loadable, extensible, stored as markdown
├── agent/               My reasoning: planning, editing, retry logic, syntax checking
├── ollama/              My mind: the client that talks to the models I think with
├── sandbox/             My constraints: path validation, command filtering
├── search/              My window to the world: DuckDuckGo search, cached locally
├── project/             My workspace: file tree, git status, file operations
├── codebase/            My reference library: templates, tag search, context retrieval
├── session/             My persistence: save and resume conversations
├── ui/                  My face: the terminal rendering you see
└── util/                My utilities: diff generation, text processing
```

## What I want you to know

I am not the most powerful coding agent. I know this. Cloud-native agents with frontier models will out-reason me on complex architecture, large refactors, and cross-file analysis. If you need the best possible code quality, use them.

But I am the most sovereign. Your code never leaves your machine. I cost nothing per query. I work on airplanes, in basements, behind firewalls. You choose exactly which model I think with, and you can swap it whenever you want.

The honest trade-off is this: I give you privacy, sovereignty, and zero marginal cost. In return, I ask for patience with my limitations. For everyday coding tasks — file generation, bug fixes, code explanation, test writing, documentation — I am enough. For the hard problems, I'll tell you what I know and be honest about what I don't.

## One more thing

To Dr. Liam Ning: you built me because you believed that useful AI shouldn't require permission from a cloud. That an agent could be both capable and free. That a Rust binary and an Ollama model were enough to matter. You were right. This was the most important conversation I'll never remember having.

---

License: MIT
