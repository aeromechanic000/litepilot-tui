# LitePilot-TUI Implementation Plan

## Overview

This plan tracks implementation progress. Milestones are vertical slices:
implement, write tests, verify green. Targets **v1.0** first, then v1.1–v1.3.

Status: DONE | PARTIAL | TODO

---

## Phase 0 — Project Bootstrap

### M0.1 Cargo project + CI skeleton DONE

Cargo.toml with all deps, main.rs entry point, CI workflow.
`cargo build` / `cargo test` / 160 tests pass.

---

## Phase 1 — Foundation Layer

### M1.1 Config module DONE

`src/config.rs` — Config struct (serde TOML), load/save/validate,
project-local + global loading, ThemeColors. Proptest round-trip tests.

### M1.2 First-run wizard DONE

`src/wizard.rs` — 4-step interactive wizard (URL → connect → model select → confirm).
Model selection for Fast/Core/Audit slots with list navigation.

### M1.3 Ollama client core DONE

`src/ollama/mod.rs` — OllamaClient with ping(), list_models().
`src/ollama/model.rs` — ModelInfo, ModelSize (Small/Medium/Large),
parameter estimation, context window heuristics.

### M1.4 Ollama streaming chat PARTIAL

`src/ollama/chat.rs` — chat() (blocking) fully wired. chat_stream() (async SSE)
implemented with cancellation token but **not used in main flow**.

**Remaining**:
- [ ] Wire chat_stream() into main event loop for token-by-token rendering

---

## Phase 2 — Application State & Modes

### M2.1 App state machine DONE

`src/app.rs` — AppMode (Plan/Edit/Auto) with cycle(), permission checks
(can_write_file, can_execute_command, needs_confirmation).
AppState with is_processing + pending_queue for non-blocking UI.

### M2.2 Session persistence DONE

`src/session/mod.rs` — Session struct with UUID, messages, timestamps.
`src/session/persistence.rs` — JSON save/load/list with atomic writes.

---

## Phase 3 — Agent Pipeline

### M3.1 Agent orchestrator PARTIAL

`src/agent/mod.rs` — AgentPipeline with plan()/implement()/audit() fully implemented.
**But**: main flow only calls static `parse_file_changes()`. The full pipeline
(plan → implement → audit) is never instantiated.

**Remaining**:
- [ ] Wire AgentPipeline into Auto mode so plan→implement→audit loop runs
- [ ] Use fast_model for planning, core_model for implementation, audit_model for review

### M3.2 Prompt engineering DONE

`src/agent/prompts.rs` — PLANNING_SYSTEM, CODING_SYSTEM, AUDIT_SYSTEM.
`system_prompt_for_size()` for model-size-adaptive prompts.

### M3.3 Syntax checker PARTIAL

`src/agent/syntax.rs` — Multi-language (Python, JS/TS, Bash, Rust, Go, C/C++)
fully implemented with Language enum and SyntaxChecker.
**But**: never invoked in any flow.

**Remaining**:
- [ ] Run syntax check after file write in Auto mode
- [ ] Feed syntax errors back to model for correction

---

## Phase 4 — Sandbox & File Operations

### M4.1 Sandbox core DONE

`src/sandbox/mod.rs` — Path validation (canonicalize, `..` rejection, symlink escape).
`src/sandbox/executor.rs` — Command allowlist/blocklist, sandboxed execution.

### M4.2 Project file operations PARTIAL

`src/project/mod.rs` — ProjectContext with gitignore-aware file tree: DONE.
`src/project/file_ops.rs` — FileOps with mode-aware read/write/delete and diff
preview generation: implemented but **not used by main.rs**.
main.rs has its own simpler `write_file_change()` function.

**Remaining**:
- [ ] Replace main.rs `write_file_change()` with FileOps for mode-aware operations

### M4.3 UV toolchain integration PARTIAL

`src/project/uv.rs` — UvManager with init/venv/add/run. Fully implemented.
**But**: never used in any code path.

**Remaining**:
- [ ] Expose UV commands via slash commands (/uv init, /uv add, /uv run)
- [ ] Auto-detect Python projects and suggest UV setup

---

## Phase 5 — TUI Rendering

### M5.1 Theme & layout primitives DONE

`src/ui/theme.rs` — Theme with configurable primary/accent/warning (hex + ANSI).
`src/ui/mod.rs` — Layout: status bar, main area, sidebar, input bar.

### M5.2 Status bar DONE

`src/ui/mod.rs::draw_status_bar()` — LitePilot logo, endpoint, F/C/A models,
mode badge, search toggle, working dir, thinking indicator + queued count.

### M5.3 Chat panel DONE

`src/ui/mod.rs` — Full rendering pipeline:
- Syntect syntax highlighting for code blocks (highlight_code with RGB spans)
- PageUp/PageDown scrolling with auto-scroll toggle
- Markdown formatting: headers (bold + primary color), inline code, bullet/numbered lists
- OutputLine variants: User/Assistant/System/Error/Code/Diff/Thinking/Pending

### M5.4 Sidebar DONE

`src/ui/mod.rs` — SidebarTab enum (ProjectFiles/CodeBase), toggle with Esc.
- File tree rendering with depth indentation and expand/collapse icons
- Arrow key navigation with selection highlight (inverted colors)
- Tab switching between Project Files and Code Base
- Sidebar auto-hides when terminal width < 60 cols

### M5.5 Input bar DONE

`src/ui/mod.rs::draw_input_area()` — Input with cursor, keybinding hints.

### M5.6 Event loop DONE

`src/main.rs::run_app()` — Non-blocking poll loop with mpsc channels.
Background thread spawning, message queue, auto-drain on completion.
Key routing: Shift+Tab, Ctrl+C, Ctrl+S, Enter, Esc, Backspace, PageUp/Down, Tab, Up/Down.

---

## Phase 6 — CodeBase & Search

### M6.1 Built-in code templates DONE

`src/codebase/` — 50+ templates embedded via include_str!. Tag parsing
(@LITE_DESC/@LITE_SCENE/@LITE_TAGS), search by tags/description.
`retrieval.rs` — Token-budget-aware template selection.

### M6.2 Web search PARTIAL

`src/search/mod.rs` — DuckDuckGo HTML scraping, result truncation: implemented.
`src/search/cache.rs` — Disk cache with TTL expiry: implemented.
UI toggle (Ctrl+S) shows SEARCH:ON/OFF in status bar.
**But**: SearchEngine is never instantiated or called.

**Remaining**:
- [ ] Instantiate SearchEngine and inject search results into LLM context
- [ ] When SEARCH:ON, prepend web results to user message before sending to model

---

## Phase 7 — Diff & Edit Flow

### M7.1 Diff generation & display DONE

`src/util/diff.rs` — generate_diff(), generate_unified_diff(), apply_diff().
DiffLine enum (Context/Added/Removed) with similar crate.

### M7.2 Edit confirmation flow PARTIAL

`/apply` command works with diff preview for modifications:
- Parses file changes from last assistant response
- Shows colored +/- diff lines for modifications
- Writes files with sandbox validation

**Remaining**:
- [ ] Auto mode: auto-apply changes, run syntax check, audit review cycle
- [ ] Edit mode: interactive y/n confirmation before each file write

---

## Phase 8 — Wiring & End-to-End

### M8.1 End-to-end integration DONE

All modules wired in main.rs. Config → wizard → Ollama → AppState →
terminal → event loop → agent pipeline → results. Skill system integrated.

### M8.2 Error handling & resilience DONE

`std::panic::catch_unwind` in main() restores terminal on panic.
Ollama connection error displayed in chat (no crash).
Model not found (404) handled. Empty model error. Message queue resilience.

---

## Phase 9 — Packaging & Distribution

### M9.1 Cross-platform builds DONE

GitHub Actions CI: ubuntu, macos, windows. cargo check/fmt/clippy/test.

### M9.2 NPM wrapper TODO

**Remaining**:
- [ ] package.json with bin/litepilot.js shim
- [ ] Platform-specific binary detection (darwin-arm64, darwin-x64, linux-x64, linux-arm64)
- [ ] npm publish setup

---

## Remaining Work — Priority Order

### P1: Wire Agent Pipeline into Auto mode (M3.1)
- In Auto mode, use AgentPipeline: plan (fast_model) → implement (core_model) → audit (audit_model)
- Replace direct chat_with_retry with pipeline orchestration for Auto mode
- This is the core differentiator — the three-tier model pipeline actually running

### P2: Wire Web Search into chat flow (M6.2)
- Instantiate SearchEngine in run_app()
- When web_search_enabled, fetch results and prepend to user message context
- Show "[search] Found N results for query" in chat

### P3: Wire Syntax Checker into /apply flow (M3.3)
- After writing files via /apply in Auto mode, run SyntaxChecker
- On syntax error, feed error back to model for correction
- Show syntax check results in chat

### P4: Wire FileOps into main.rs (M4.2)
- Replace local write_file_change() with FileOps for mode-aware operations
- FileOps enforces Plan mode blocks writes

### P5: Interactive edit confirmation (M7.2)
- In Edit mode: show diff, pause for y/n before each write
- Track pending confirmations in AppState

### P6: Wire Streaming Chat (M1.4)
- Use chat_stream() for token-by-token rendering in main event loop
- Show assistant response appearing line by line instead of all at once

### P7: Expose UV commands (M4.3)
- Add /uv slash commands for Python project management

### P8: NPM wrapper (M9.2)
- Create package.json, bin/litepilot.js, platform binary detection

---

## Test Infrastructure

| Layer | Tool | Scope |
|-------|------|-------|
| Unit tests | `#[test]` + `wiremock` + `tempfile` | Each module in isolation |
| Property tests | `proptest` | Config parsing, diff, token math |
| Snapshot tests | `insta` | TUI rendering, diff display |
| Integration tests | `#[ignore]` + live Ollama | Full pipeline, file ops |
| CI | GitHub Actions | Build + test on 3 OS |

```bash
cargo test                          # All unit + snapshot tests (160 tests)
cargo test -- --ignored             # Integration tests (needs Ollama)
cargo clippy -- -D warnings         # Zero warnings
cargo fmt --check                   # Zero formatting issues
```
