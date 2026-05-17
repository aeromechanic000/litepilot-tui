# litecode-tui

Terminal AI coding assistant powered by Ollama-host local models. Written in Rust.

No API keys required. No cloud dependencies. Everything runs locally.

## Features

- **Local-first**: Uses Ollama-host models (3-5B fast, 6-7B core, 7-14B audit)
- **Three modes**: Plan (read-only) → Edit (confirm each write) → Auto (sandboxed full-auto)
- **Skills system**: Built-in slash commands (`/search`, `/review`, `/explain`, `/simplify`, `/test`)
- **Knowledge search**: No embeddings needed — uses `grep`/`find`/`cat` to search local files
- **Auto-retry**: Validates model responses and retries with error context for small models
- **Dark & light theme**: Auto-detects terminal background

## Quick Start

### Prerequisites

- [Ollama](https://ollama.ai) running locally with at least one model pulled
- Rust toolchain (`rustup`)

### Build & Run

```bash
# Build
cargo build --release

# Run in current directory
cargo run

# Run in a specific directory
cargo run -- -d /path/to/project
```

### One-Command Install

```bash
cargo install --path .
```

Then use `litecode` from anywhere:

```bash
litecode              # Run in current directory
litecode -d ~/myproj  # Run in a specific project
```

## Configuration

Config lives at `~/.litecode/config.toml`. On first run, a setup wizard creates defaults.

```toml
ollama_endpoint = "http://127.0.0.1:11434"
fast_model = "qwen3:4b"
core_model = "qwen3:8b"
audit_model = "qwen3:14b"
default_mode = "edit"
max_retries = 3
```

### Project-Local Config

Place a `.litecode/config.toml` in your project root to override global settings for that project.

## Skills

Skills are task-specific guides invoked via slash commands. Built-in skills are populated to `~/.litecode/skills/` on first run. You can add custom skills as markdown files.

| Command | Description |
|---------|-------------|
| `/search <query>` | Search local files using grep/find/cat (no embeddings) |
| `/review <code>` | Review code for bugs, style, and security |
| `/explain <code>` | Explain code in plain language |
| `/simplify <code>` | Simplify and refactor code |
| `/test <code>` | Generate comprehensive tests |
| `/skills` | List all available skills |

### Custom Skills

Create a markdown file in `~/.litecode/skills/` with YAML frontmatter:

```markdown
---
name: my-skill
description: What this skill does
trigger: keyword1, keyword2
---

Your skill prompt goes here.
This guides the model when /my-skill is invoked.
```

## Key Bindings

| Key | Action |
|-----|--------|
| `Enter` | Send input |
| `Shift+Tab` | Switch mode (Plan → Edit → Auto) |
| `Ctrl+S` | Toggle web search |
| `Ctrl+C` | Quit |
| `Esc` | Toggle sidebar |

## Publish to npm

You can wrap the compiled binary as an npm package so users can install and run it with a single `npx` command from any directory.

### 1. Build the binary

```bash
cargo build --release
```

The binary is at `target/release/litecode`.

### 2. Create the npm wrapper

Create a `package.json` in the project root:

```json
{
  "name": "litecode-tui",
  "version": "0.1.0",
  "description": "Terminal AI coding assistant powered by local Ollama models",
  "bin": {
    "litecode": "./bin/litecode.js"
  },
  "files": [
    "bin/",
    "README.md"
  ],
  "os": [
    "darwin",
    "linux"
  ],
  "cpu": [
    "x64",
    "arm64"
  ],
  "keywords": ["tui", "ai", "coding", "assistant", "ollama", "local", "terminal"],
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/aeromechanic000/litecode-tui"
  }
}
```

Create `bin/litecode.js` as the entry point:

```javascript
#!/usr/bin/env node
const { execFileSync } = require('child_process');
const path = require('path');

const platform = process.platform;
const arch = process.arch;

let binaryName;
if (platform === 'darwin' && arch === 'arm64') {
  binaryName = 'litecode-darwin-arm64';
} else if (platform === 'darwin' && arch === 'x64') {
  binaryName = 'litecode-darwin-x64';
} else if (platform === 'linux' && arch === 'x64') {
  binaryName = 'litecode-linux-x64';
} else if (platform === 'linux' && arch === 'arm64') {
  binaryName = 'litecode-linux-arm64';
} else {
  console.error(`Unsupported platform: ${platform}-${arch}`);
  process.exit(1);
}

const binaryPath = path.join(__dirname, binaryName);
const args = process.argv.slice(2);

try {
  execFileSync(binaryPath, args, { stdio: 'inherit' });
} catch (e) {
  process.exit(e.status || 1);
}
```

### 3. Build platform binaries

Build for each target platform:

```bash
# macOS ARM64 (Apple Silicon)
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# macOS x64 (Intel)
rustup target add x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin

# Linux x64
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu

# Linux ARM64
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

Copy the binaries into `bin/`:

```bash
mkdir -p bin
cp target/aarch64-apple-darwin/release/litecode bin/litecode-darwin-arm64
cp target/x86_64-apple-darwin/release/litecode bin/litecode-darwin-x64
cp target/x86_64-unknown-linux-gnu/release/litecode bin/litecode-linux-x64
cp target/aarch64-unknown-linux-gnu/release/litecode bin/litecode-linux-arm64
chmod +x bin/litecode-*
```

### 4. Publish

```bash
npm login
npm publish
```

### 5. Use from anywhere

After publishing, anyone can run it without cloning the repo:

```bash
# Run without installing
npx litecode-tui

# Or install globally
npm install -g litecode-tui
litecode

# Run in any project directory
cd ~/my-project
npx litecode-tui

# Specify a directory
npx litecode-tui -- -d /path/to/project
```

## Architecture

```
src/
├── main.rs              Entry point, CLI args, event loop
├── app.rs               Application state machine (AppState, modes)
├── config.rs            config.toml parsing, defaults, first-run wizard
├── skills/              Skills system (~/.litecode/skills/)
│   ├── mod.rs           SkillRegistry, Skill struct
│   ├── parser.rs        Markdown + YAML frontmatter parser
│   └── builtin.rs       Built-in skill population
├── agent/
│   ├── mod.rs           Agent orchestrator, three-model pipeline
│   ├── planner.rs       Plan mode — read-only analysis
│   ├── editor.rs        Edit mode — generate + diff preview
│   ├── auto.rs          Auto mode — full pipeline with sandbox
│   ├── retry.rs         Response validation + auto-retry with error feedback
│   ├── syntax.rs        Multi-language syntax checker
│   └── prompts.rs       System prompts per model size
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
