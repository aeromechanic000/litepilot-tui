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

### M1.4 Ollama streaming chat DONE

`src/ollama/chat.rs` — chat() (blocking) and chat_stream() (async SSE) both wired.
Streaming used in `spawn_llm_request()` for token-by-token rendering.
Cancellation token and `StreamChunk`/`StreamDone` event handling fully implemented.

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

### M3.1 Agent orchestrator DONE

`src/agent/mod.rs` — AgentPipeline with plan()/implement()/audit() fully implemented.
Auto mode uses the tool-use agent loop (`agent_loop.rs`) instead of the classical
plan→implement→audit pipeline. Both approaches coexist. The agent loop is the
primary path; AgentPipeline available for explicit plan-based workflows.

### M3.2 Prompt engineering DONE

`src/agent/prompts.rs` — PLANNING_SYSTEM, CODING_SYSTEM, AUDIT_SYSTEM.
`system_prompt_for_size()` for model-size-adaptive prompts.

### M3.3 Syntax checker DONE

`src/agent/syntax.rs` — Multi-language (Python, JS/TS, Bash, Rust, Go, C/C++)
fully implemented with Language enum and SyntaxChecker.
Run after every file write in Auto mode via `run_syntax_check()`.
Diagnostic-based self-correction added in M10.5.5.

---

## Phase 4 — Sandbox & File Operations

### M4.1 Sandbox core DONE

`src/sandbox/mod.rs` — Path validation (canonicalize, `..` rejection, symlink escape).
`src/sandbox/executor.rs` — Command allowlist/blocklist, sandboxed execution.

### M4.2 Project file operations DONE

`src/project/file_ops.rs` — FileOps with mode-aware read/write/delete and diff
preview generation. Fully integrated in main.rs for all file operations.
Sandbox validation applied to every write.

### M4.3 UV toolchain integration DONE

`src/project/uv.rs` — UvManager with init/venv/add/run. Exposed via `/uv` slash
commands (`/uv init`, `/uv venv`, `/uv add`, `/uv run`).

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

### M6.2 Web search DONE

`src/search/mod.rs` — DuckDuckGo HTML scraping, result truncation.
`src/search/cache.rs` — Disk cache with TTL expiry.
UI toggle (Ctrl+S) shows SEARCH:ON/OFF in status bar.
When enabled, web search runs automatically and results injected into LLM context.
`WebSearch` tool registered in ToolRegistry for agent loop access.

---

## Phase 7 — Diff & Edit Flow

### M7.1 Diff generation & display DONE

`src/util/diff.rs` — generate_diff(), generate_unified_diff(), apply_diff().
DiffLine enum (Context/Added/Removed) with similar crate.

### M7.2 Edit confirmation flow DONE

`/apply` command with diff preview, interactive y/n/a confirmation per file.
Auto mode: auto-apply changes, run syntax check, diagnostic self-correction.
Edit mode: interactive y/n/a confirmation before each file write.

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
---

## Phase 10 — Agent Quality (v2.0)

The remaining phases (10-16) address the architectural gap between LitePilot
and production coding agents (DeepSeek-TUI, Claude Code, Codex). They are
organized by priority: P0 changes the core agent architecture, P1 improves
daily usability, P2 hardens the system, P3 adds advanced features.

Reference: `notes/deepseek-tui-agent-design.md`

---

### M10.1 Tool-Use Protocol & Agent Loop — P0 DONE

**Tasks:**
- [x] Create `src/tools/mod.rs` — `Tool` trait, `ToolDef`, `ToolResult`
- [x] Implement built-in tools: `read_file`, `write_file`, `edit_file`, `list_dir`, `exec_shell`, `web_search`
- [x] Create `src/agent/agent_loop.rs` — agent loop with max_steps guard
- [x] Wire Ollama tool definitions into `ChatRequest` via `chat_with_tools()`
- [x] Parse tool_use response blocks (JSON + text fallback) and dispatch to tools
- [x] Feed `tool_result` messages back into conversation for next LLM call
- [x] Add loop guard: detect and break on identical tool calls repeating
- [x] Register all tools (file ops, shell, web search) in `ToolRegistry`

---

### M10.2 Layered System Prompt Assembly — P0 DONE

**Tasks:**
- [x] Create `src/prompt.rs` — `PromptBuilder` struct
- [x] Define layers: base identity → mode overlay → skills → project context (static prefix)
- [x] Define volatile tail: working set summary + conversation summary + date/time
- [x] Store `PromptBuilder` in `AppState`, rebuild only when mode/skills/project changes
- [x] Inject project context (AGENTS.md / CLAUDE.md / .litepilot/instructions.md)
- [x] Add environment block: platform, version, shell, working directory
- [x] Preserve byte-identical prefix across turns for cache hits
- [x] Add `RECAP_SYSTEM` constant for post-summarization context continuity

---

### M10.3 Context Compaction with Summarization — P0 DONE

**Tasks:**
- [x] Create `src/agent/summarizer.rs` with `SummarizerConfig`, `SummaryResult`, `MessagePriority`
- [x] Add message pinning: error messages, file paths, code patches never summarized
- [x] Add `MessagePriority` enum (Normal, Pinned)
- [x] Implement `needs_summarization()` capacity checker
- [x] Implement `summarize()` using fast_model for background summarization
- [x] Add `compact_with_summary()` to `src/context.rs` — replaces truncation with LLM-powered compaction
- [x] Store conversation summary in `AppState` for injection into system prompt
- [x] Trigger summarization after `StreamDone` when >80% context used

---

## Phase 10.5 — Small-Model Cognitive Scaffolding (v2.05)

Reference: `notes/challenges-and-ideas-to-small-model-coding-agents.md`

Small models (4B-14B) need "cognitive scaffolding" to overcome three bottlenecks:
context window scarcity (lost-in-the-middle), instruction drift, and tool-use
reliability. Phase 10 built the foundations; this phase adds the scaffolding
that makes those foundations actually work reliably with small models.

---

### M10.5.1 Edge-Aware Prompt Construction — P0 DONE

**Tasks:**
- [x] Add `current_goal: Option<String>` and `completed_tasks: Vec<String>` to `PromptBuilder`
- [x] Set `current_goal` from the user's first message each turn (extracted in `spawn_request_for_mode`)
- [x] Add goal re-injection in `PromptBuilder::build()` volatile tail — places `## Current Objective` right before the user request
- [x] After `compact_with_summary()`, inject `[CURRENT OBJECTIVE]` + project instructions as the first history message after the system prompt
- [x] Add unit tests for goal placement and re-injection after summarization

**Files:** `src/prompt.rs`, `src/context.rs`, `src/main.rs`

---

### M10.5.2 Tool-Use Hardening for Small Models — P0 DONE

**Completed:**
- [x] Add `ToolRegistry::list_names()` returning all registered tool names
- [x] Add `ToolRegistry::has_tool()` and `validate_params()` — validate name + required fields
- [x] In `run_agent_loop()`, validate tool name exists before executing; if not, inject error with available tool list
- [x] Validate required parameters before execution; inject specific missing-param errors
- [x] Add `TOOL_CORRECTION_PROMPT` to `src/agent/prompts.rs` — shows both JSON and text format examples
- [x] Add `looks_like_failed_tool_call()` detection for malformed attempts
- [x] Modify `parse_tool_calls()` to return `ParseResult` with diagnostic info (ParseDiagnostics with hints_found + failure_reasons)
- [x] In `run_agent_loop()`, when parse fails but looks like a tool attempt, inject correction + continue loop (max 2 retries)
- [x] Add `REFLEXION_PROMPT` — on final retry attempt, ask model to verbalize what went wrong
- [x] Add unit tests for parse diagnostics, failed attempt detection, and correction formatting

---

### M10.5.3 Hierarchical Planning for Instruction Drift — P1 DONE

**Problem:** Planning is single-level (flat list of steps). Small models
become reactive to the most recent error instead of following a strategic
plan. No mechanism detects when the model has drifted from the original goal.

**Target:** Two-phase planning: strategic goal + operational steps. The
strategic goal is re-injected into every step's context. Drift detection
checks if the current response is still relevant to the active phase.

**Tasks:**
- [x] Extend `Plan` struct with `strategic_goal: String` field (the one-line user objective)
- [x] Modify planner prompt to extract strategic goal as first line, then operational steps
- [x] In plan-based execution, inject `[STRATEGIC GOAL: ...]` into each step's system context
- [x] Add `detect_drift(goal: &str, response: &str) -> bool` — checks if response mentions topics unrelated to goal (simple keyword overlap heuristic)
- [x] On drift detection, inject a warning: "You are drifting from the objective. Refocus on: {goal}"
- [x] Add unit tests for drift detection and goal re-injection

**Files:** `src/agent/planner.rs`, `src/agent/mod.rs`, `src/agent/retry.rs`

---

### M10.5.4 Semantic Reranking for Template Retrieval — P1 DONE

**Problem:** `codebase/retrieval.rs` uses a single LLM call to select templates
from a catalog. No two-stage retrieve-then-rerank pipeline. For small models,
noisy context (irrelevant templates) wastes precious context window and
degrades output quality.

**Target:** Two-stage retrieval: (1) broad candidate selection from catalog,
(2) fast_model reranking with code-aware scoring. Only top-K within budget
are injected into context.

**Tasks:**
- [x] Add `retrieve_with_reranking()` to `src/codebase/retrieval.rs`
- [x] Stage 1: existing `select()` call returns broad candidate set (top 10)
- [x] Stage 2: build rerank prompt with candidate code snippets (first 500 chars each), ask fast_model to rank by semantic relevance
- [x] Load only top-K reranked templates within token budget
- [x] Fall back to existing `retrieve()` if reranking fails (non-blocking)
- [x] Add `RERANK_SYSTEM` prompt to `src/agent/prompts.rs`
- [x] Add unit tests for reranking prompt construction and fallback behavior

**Files:** `src/codebase/retrieval.rs`, `src/agent/prompts.rs`

---

### M10.5.5 Diagnostic-Based Self-Correction — P1 DONE

**Tasks:**
- [x] Create `src/agent/diagnostics.rs` — `run_diagnostics(path, sandbox) -> DiagnosticResult`
- [x] `DiagnosticResult` contains `errors: Vec<DiagnosticError>` with file, line, message
- [x] After `auto_apply_changes()`, spawn background diagnostic run on written files
- [x] Send `DiagnosticReady` via channel; event loop displays errors in UI
- [x] `DiagnosticResult::format_for_correction()` builds correction prompt from actual errors
- [x] Non-blocking: diagnostic failure doesn't block the agent loop, just skips correction
- [x] Add `DIAGNOSTIC_CORRECTION_PROMPT` to `src/agent/prompts.rs`
- [x] Add unit tests for diagnostic result formatting and correction prompt

**Files:** `src/agent/diagnostics.rs`, `src/agent/prompts.rs`, `src/main.rs`

---

## Phase 11 — Usability (v2.1)

---

### M11.1 Working Set Tracking — P1 DONE

**Tasks:**
- [x] Create `src/working_set.rs` — `WorkingSet` with frecency-based ranking
- [x] Hook into file write/read tool results to observe paths
- [x] Prune to max 20 entries by frequency + recency
- [x] Inject `working_set.summary()` into volatile section of system prompt

---

### M11.2 Session Resume & Auto-Save — P1 DONE

**Tasks:**
- [x] Auto-save session after each StreamDone / RetryResult::Success
- [x] Add `--resume` and `--resume <id>` CLI flags
- [x] Add `--sessions` flag to list sessions
- [x] Load conversation history from resumed session into AppState

---

### M11.3 Project Context Auto-Discovery — P1 DONE

**Tasks:**
- [x] Search priority: AGENTS.md → CLAUDE.md → .litepilot/instructions.md → README.md
- [x] Auto-generate instructions from: project name, language, structure, build/test commands
- [x] Save auto-generated `.litepilot/instructions.md` for user customization
- [x] Inject discovered context into PromptBuilder static layer

---

### M11.4 Optimized First-Run Initialization — P1 DONE

**Tasks:**
- [x] Ollama ping already runs in background thread
- [x] Add crash dump handler: `std::panic::set_hook` → write to `~/.litepilot/crashes/`

---

### M11.5 Session Recap — P1 DONE

**Tasks:**
- [x] Create `src/recap.rs` — `generate_recap(client, messages, config) -> Result<String>`
- [x] Add `/recap` slash command handler in `main.rs` event loop
- [x] Add end-of-turn recap after Auto mode with >2 file changes (guard with config flag)
- [x] Add config flags: `enable_recap`, `enable_away_summary` in `src/config.rs`

**Deferred:**
- Away summary on terminal focus regain (requires complex crossterm focus event tracking)

---

## Phase 12 — Production Hardening (v2.2)

---

### M12.1 Multi-Provider LLM Client — DROPPED

LitePilot is local-first with Ollama. Multi-provider support is out of scope.

---

### M12.2 Streaming Guardrails & Resilient Transport — P2 DONE

**Tasks:**
- [x] Add `MAX_CONTENT_BYTES` (10 MB) guard in `chat_stream()`
- [x] Add `MAX_DURATION` (30 min) wall-clock limit
- [x] Add `MAX_ERRORS` (5) error tolerance — stream read errors and JSON parse errors tolerated individually
- [x] Error counter resets per-category, `total_bytes` tracks cumulative content size
- [x] Wall-clock timeout checked each loop iteration via `std::time::Instant`

**Files:** `src/ollama/chat.rs`

---

### M12.3 Risk-Classified Approval System — P2 DONE

**Tasks:**
- [x] Create `src/approval.rs` — `RiskLevel` (Safe, Write, Destructive)
- [x] Classify tools: read_file/list_dir = safe, write_file/edit_file = write, exec_shell = side-effect
- [x] Destructive operations (delete, rm) require two-key confirmation (YY)
- [x] Add `ApprovalCache`: HashSet of approved tool signatures, auto-approve for session
- [x] Skip approval for cached items — show "[cached]" prefix on auto-approved
- [x] `ApprovalCache` stored in `AppState`, persists for entire session
- [x] 14 unit tests for classification, caching, and decision logic

**Files:** `src/approval.rs`, `src/app.rs`, `src/main.rs`

---

### M12.4 Auto Model Routing — P2 DONE

**Tasks:**
- [x] Create `src/router.rs` — `classify_request()` heuristic
- [x] Keywords: question words → Fast, "create/fix/implement" → Core, "review/audit/check" → Audit
- [x] Add config flag: `auto_model_routing = false` (opt-in)
- [x] Respect manual model selection when `auto_model_routing = false`

**Files:** `src/router.rs`, `src/main.rs`, `src/config.rs`

---

## Phase 13 — Advanced Features (v2.3)

---

### M13.1 Workspace Snapshots (Side-Git) — P3 DONE

**Tasks:**
- [x] Create `src/snapshot.rs` — `SnapshotManager`
- [x] Implement `pre_turn()` / `post_turn()` using side git
- [x] Add retention: 7 days, 50 snapshots max, `prune()` method
- [x] Add `/undo`, `/restore <hash>`, `/snapshots` slash commands
- [x] Non-fatal: snapshot failures never block TUI operation
- [x] Pre-turn snapshot before each LLM request; post-turn after Auto mode changes
- [x] 8 unit tests (init, create, restore, hash stability)

**Files:** `src/snapshot.rs`, `src/app.rs`, `src/main.rs`

---

### M13.2 Structured Event Hooks — P3 DONE

**Tasks:**
- [x] Create `src/hooks.rs` — `HookEvent` enum (TurnStarted, ToolCalled, ToolResult, TurnComplete, Error)
- [x] Implement `JsonlSink` → write to `~/.litepilot/logs/events.jsonl`
- [x] Emit TurnStarted on each LLM request, TurnComplete on result
- [x] Emit Error events on pipeline failures
- [x] JSONL format: one JSON object per line, tagged with `type` field
- [x] 8 unit tests (serialization, file writing, append mode, directory creation)

**Files:** `src/hooks.rs`, `src/app.rs`, `src/main.rs`

---

### M13.3 OS-Level Sandboxing — P3 DONE

**Tasks:**
- [x] Create `src/sandbox/seatbelt.rs` — macOS `sandbox-exec` integration
- [x] Create `src/sandbox/landlock.rs` — Linux Landlock placeholder with kernel version detection
- [x] Build Seatbelt sandbox policy: allow read from /usr, read/write from workspace, allow network
- [x] Add `run_os_sandboxed()` to Executor — opt-in OS-level isolation
- [x] Fallback to allowlist/blocklist on unsupported platforms
- [x] 8 new tests (profile generation, availability checks, kernel version parsing)

**Files:** `src/sandbox/seatbelt.rs`, `src/sandbox/landlock.rs`, `src/sandbox/executor.rs`, `src/sandbox/mod.rs`

---

### M13.4 LSP Post-Edit Diagnostics — P3 DONE

**Tasks:**
- [x] Create `src/lsp.rs` — lightweight LSP client over stdio (JSON-RPC)
- [x] Auto-detect language server from file extension (rs→rust-analyzer, py→pyright, ts/tsx→typescript-language-server)
- [x] After file writes in Auto mode and cached approvals, query LSP diagnostics
- [x] Display diagnostics in TUI as System messages with line, severity, and message
- [x] Non-blocking: LSP failure doesn't block tool result, silently skipped
- [x] LSP client spawns server, sends initialize/didOpen, reads diagnostics, shuts down
- [x] 5 unit tests (file detection, URI format, constructors)

**Files:** `src/lsp.rs`, `src/main.rs`

---

## Implementation Priority Summary

| Priority | Milestones | Impact |
|----------|-----------|--------|
| **P0** | M10.1 (Tool-use + agent loop), M10.2 (Layered prompts), M10.3 (Summarization) | Core agent quality — DONE |
| **P0** | M10.5.1 (Edge-aware prompts), M10.5.2 (Tool-use hardening + correction retry) | Small-model reliability — DONE |
| **P1** | M10.5.3 (Hierarchical planning), M10.5.4 (Semantic reranking), M10.5.5 (Diagnostic self-correction) | Small-model quality — DONE |
| **P1** | M11.1 (Working set), M11.2 (Session resume), M11.3 (Project context), M11.4 (Init optimization), M11.5 (Session recap) | Daily usability — DONE |
| **P2** | M12.2 (Stream guardrails), M12.3 (Risk approval), M12.4 (Auto routing) | Production hardening — DONE |
| **P3** | M13.1 (Snapshots), M13.2 (Hooks), M13.3 (OS sandbox), M13.4 (LSP) | Enterprise-grade features — DONE |

**Recommended order:** M10.5.1 → M10.5.2 (P0 scaffolding) → M10.5.3 →
M10.5.4 → M10.5.5 (P1 scaffolding) → Phase 11 (usability). The scaffolding
builds on Phase 10's foundations and makes them actually reliable for small models.

---

## The Five Core Patterns

All production coding agents share these patterns. LitePilot needs all five
to close the gap:

1. **Tool-use loop** — LLM calls tools, sees results, decides next action.
   Without this, you have a chatbot, not an agent.

2. **Context preservation** — Summarization + working set + project context.
   The agent remembers what it did, even in long sessions.

3. **Cache-stable prompts** — Layered prompts with byte-identical prefix.
   Critical for local model performance (Ollama KV cache reuse).

4. **Resilient transport** — Stream guardrails, transparent retry, error
   classification. The agent doesn't crash on network hiccups.

5. **Safety nets** — Snapshots for undo, risk-classified approval, OS
   sandboxing. Users trust the agent when they can undo its mistakes.

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
