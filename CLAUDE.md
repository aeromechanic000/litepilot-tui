# LiteCode-TUI

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
├── main.rs              Entry point, CLI args, bootstrap
├── app.rs               Application state machine (AppState, modes)
├── config.rs            config.toml parsing, defaults, first-run wizard
├── ui/
│   ├── mod.rs           TUI rendering dispatch
│   ├── layout.rs        Panel layout, status bar, input area
│   ├── theme.rs         Color palette (DeepSeek blue theme)
│   ├── chat.rs          Central output panel (streaming, diff view)
│   ├── sidebar.rs       File tree + code_base browser
│   └── input.rs         Bottom input bar, keybinding hints
├── ollama/
│   ├── mod.rs           Ollama client (connectivity, model list)
│   ├── chat.rs          Streaming chat completion (/api/chat)
│   └── model.rs         Model info, size classification, context strategy
├── agent/
│   ├── mod.rs           Agent orchestrator, three-model pipeline
│   ├── planner.rs       Plan mode — read-only analysis & architecture
│   ├── editor.rs        Edit mode — generate + diff preview + confirm
│   ├── auto.rs          Auto mode — full pipeline with sandbox
│   └── syntax.rs        Multi-language syntax checker dispatch
├── sandbox/
│   ├── mod.rs           Path validation, command allowlist
│   └── executor.rs      Sandboxed command runner
├── search/
│   ├── mod.rs           Free web search, region detection, caching
│   └── cache.rs         Local disk cache for search results
├── project/
│   ├── mod.rs           Project context (file tree, git status)
│   ├── file_ops.rs      Read/write/delete with sandbox checks
│   └── uv.rs            UV toolchain integration (init, venv, add, run)
├── codebase/
│   ├── mod.rs           Built-in code template library
│   └── index.rs         Tag-based lightweight RAG matching
├── session/
│   ├── mod.rs           Conversation session management
│   └── persistence.rs   Serialize/deserialize sessions to disk
└── util/
    ├── mod.rs
    ├── diff.rs          Diff generation & display
    └── text.rs          Text truncation, token estimation
```

## Key Design Decisions

- **Rust + ratatui + crossterm + tokio**: Single binary, cross-platform, low resource usage.
- **Three-tier models**: Fast (3-5B, planning), Core (6-7B, coding), Audit (7-14B, review). All via Ollama `/api/chat` streaming.
- **Three modes**: Plan (read-only) → Edit (confirm each write) → Auto (sandboxed full-auto). Toggle with `Shift+Tab`.
- **Sandbox**: Auto mode locks all file ops and command execution to the startup working directory. Allowlist-based command filtering.
- **Offline-first**: Built-in `~/.litecode/code_base/` templates with `@LITE_*` tags for lightweight RAG. Web search is optional.
- **Config**: `~/.litecode/config.toml` (TOML). First-run wizard configures Ollama endpoint + model selection.
- **No external API keys**: Web search uses public search engines with region auto-detection and local caching.

## Naming Conventions

- Package/binary: `litecode-tui` / `litecode`
- User config dir: `~/.litecode/`
- Avoid referencing competitor product names in code or UI text.

## Testing Strategy

- **Unit tests**: Each module has `#[cfg(test)] mod tests` inline. Mock Ollama responses with `tokio::test` + wiremock.
- **Integration tests**: `tests/` directory. Tests needing a live Ollama are marked `#[ignore]`.
- **TUI snapshot tests**: Use `insta` for rendered terminal output snapshots.
- **Sandbox tests**: Verify path traversal blocking, command allowlist enforcement.
- **Property tests**: `proptest` for config parsing, diff generation, token estimation.

## Dependencies (key)

- `ratatui` + `crossterm` — TUI rendering
- `tokio` — async runtime
- `reqwest` — HTTP client for Ollama API
- `serde` + `toml` — config serialization
- `walkdir` — file tree traversal
- `similar` — diff generation
- `insta` — snapshot testing
- `wiremock` — HTTP mocking in tests
- `proptest` — property-based testing
- `tempfile` — test fixtures

## Version Roadmap

- **v1.0**: TUI shell, Ollama connection, 3-model config, mode switching, basic file edit + diff, code_base, session save
- **v1.1**: Syntax auto-check, UV integration, model-size-adaptive prompts, Ollama error classification
- **v1.2**: Free web search + cache, sandbox hardening, advanced templates
- **v1.3**: Cross-platform builds, NPM wrapper, docs, bug fixes
