# DeepSeek-TUI Coding Agent — Architecture & Workflow

A comprehensive analysis of how DeepSeek-TUI is designed as a terminal-based AI coding assistant, covering the full lifecycle from initialization through action delivery.

---

## 1. Project Overview

DeepSeek-TUI is a Rust workspace of **17 crates** that together form a production-grade coding agent with native DeepSeek V4 integration. It supports multiple LLM providers, real-time streaming, sandboxed tool execution, sub-agent orchestration, and persistent sessions.

### Crate Map

| Crate | Role |
|-------|------|
| `cli` | Command-line dispatcher; finds and delegates to `deepseek-tui` binary |
| `tui` | Main TUI runtime — engine, tools, rendering, skills, sandbox (~272 source files) |
| `tui-core` | Event-driven TUI state machine scaffold |
| `core` | Agent loop, session/thread management, turn orchestration |
| `agent` | Model/provider registry for resolving model IDs to endpoints |
| `config` | Configuration loading with profiles, env var precedence, secret store |
| `execpolicy` | Approval/sandbox policy engine with hierarchical rulesets |
| `hooks` | Lifecycle hooks — stdout, JSONL file, HTTP webhook |
| `mcp` | MCP client + stdio server for Model Context Protocol |
| `protocol` | Request/response framing and protocol types |
| `secrets` | OS keyring integration for API key storage |
| `state` | SQLite-based thread/session persistence |
| `tools` | Shared tool invocation primitives |

---

## 2. Initialization — From CLI to Ready State

### 2.1 Console Bootstrap

```
deepseek [options] [prompt]
  │
  ├─ Set UTF-8 stdout/stderr encoding
  ├─ Install panic hook → write crash dumps to ~/.deepseek/crashes/
  ├─ Install signal handlers (SIGINT/SIGTERM/SIGHUP) → restore terminal
  └─ Parse CLI args via clap
```

The CLI dispatcher (`crates/cli/src/main.rs`) uses a **sibling binary discovery** pattern — the `deepseek` command locates its companion `deepseek-tui` binary and delegates execution.

### 2.2 Configuration Resolution

Configuration follows a strict precedence chain:

```
CLI flags → Environment variables → Project config (.deepseek/config.toml)
          → User config (~/.deepseek/config.toml) → Built-in defaults
```

**Key config fields:**
- `provider` — which LLM backend (DeepSeek, NVIDIA NIM, OpenAI, Ollama, etc.)
- `model` — model ID or `auto` for automatic routing
- `api_key` — resolved from secret store, env var, or config file
- `approval_policy` — tool approval behavior
- `sandbox_mode` — sandboxing strictness

**Provider matrix** supports: DeepSeek, NVIDIA NIM, OpenAI-compatible, AtlasCloud, OpenRouter, Novita, Fireworks, SGLang, vLLM, Ollama. Each provider has capability metadata (context window, max output, thinking support, cache telemetry).

### 2.3 Onboarding Flow (First Run)

If no configuration exists, a guided onboarding sequence runs:

```
Welcome → Language Selection → API Key Entry → Trust Directory → Tips → Done
```

Onboarding state is tracked via `OnboardingState` enum with states: `Welcome`, `Language`, `ApiKey`, `TrustDirectory`, `Tips`, `None` (completed).

### 2.4 Engine Construction

The engine is the central orchestrator. It is constructed and spawned as a background task:

```rust
pub struct Engine {
    config: EngineConfig,
    deepseek_client: Option<DeepSeekClient>,
    session: Session,
    subagent_manager: SharedSubAgentManager,
    shell_manager: SharedShellManager,
    mcp_pool: Option<Arc<AsyncMutex<McpPool>>>,
    rx_op: mpsc::Receiver<Op>,           // receives operations from UI
    tx_event: mpsc::Sender<Event>,        // sends events to UI
    cancel_token: CancellationToken,
    capacity_controller: CapacityController,
    seam_manager: Option<SeamManager>,
    lsp_manager: Arc<LspManager>,
    workshop_vars: Option<Arc<Mutex<WorkshopVariables>>>,
    sandbox_backend: Option<Arc<dyn SandboxBackend>>,
}
```

**EngineConfig** carries everything the engine needs:
- Model, workspace path, shell access, trust mode
- Notes path, MCP config path, skills directory, instruction files
- Limits: `max_steps`, `max_subagents`, `max_spawn_depth`
- Features, compaction config, cycle config, capacity config
- Network policy, LSP config, snapshot settings

### 2.5 Skill Loading

Skills are prompt templates that specialize the LLM's behavior. The discovery order is:

```
.agents/skills/ → skills/ → .deepseek/skills/
```

Each skill is a markdown file with YAML frontmatter:

```markdown
---
name: review
description: Review code for bugs, style, and security
trigger: review, code review, audit
---
Review the following code for potential bugs...
```

Skills can be installed from GitHub via `/skill install`. Community and system skills are managed through a `SkillStateStore`. Active skills are injected into the system prompt at assembly time.

### 2.6 MCP Server Startup

MCP (Model Context Protocol) servers are configured in the config file and started during initialization:

```
McpManager::start_all()
  → For each configured server:
    ├─ Spawn child process (stdio transport)
    ├─ Exchange capabilities (initialize handshake)
    ├─ Discover tools → register as mcp__{server}__{tool}
    ├─ Apply allow/deny tool filters
    └─ Report status: Starting → Ready | Failed | Cancelled
```

### 2.7 Project Context Loading

The system searches for project context files in priority order:

```
AGENTS.md → .claude/instructions.md → CLAUDE.md → .deepseek/instructions.md
```

Plus parent directory walk for monorepo support, and a global `~/.deepseek/AGENTS.md`. If no context file exists, one is auto-generated from the project structure. Maximum context file size: 100KB.

### 2.8 State Restoration

The SQLite state store (`crates/state`) is used to restore previous sessions:

```
~/.deepseek/state.db
  ├─ threads     — session metadata with git context, archival status
  ├─ messages    — full message history
  ├─ checkpoints — crash recovery snapshots
  └─ jobs        — background job tracking
```

Sessions can be resumed via `deepseek --resume` or the session picker.

---

## 3. The Core Agent Loop

### 3.1 Event-Driven Architecture

The engine and UI communicate through two channels:

```
┌──────────┐    Op (operation)     ┌──────────┐
│          │ ─────────────────────► │          │
│   TUI    │                       │  Engine  │
│   (UI)   │ ◄───────────────────── │  (bg)    │
│          │    Event (updates)     │          │
└──────────┘                       └──────────┘
```

**Op** (operations from UI → Engine):
- `SendMessage { content, mode, model, ... }`
- `CancelRequest`
- `ApproveToolCall { id }` / `DenyToolCall { id }`
- `SpawnSubAgent { prompt }`
- `ChangeMode { mode }`
- `SetModel { model }`

**Event** (updates from Engine → UI):
- `TurnStarted` / `TurnComplete` — turn lifecycle
- `Status(String)` — streaming text
- `Error(ErrorEnvelope)` — errors with category and fatality
- `ToolCallStarted` / `ToolCallResult` — tool execution
- `SessionUpdated` — authoritative API state
- `ApprovalRequired` — tool needs user approval
- `UserInputRequired` — tool needs live user input
- `ElevationRequired` — sandbox denial

### 3.2 The Turn Loop

A "turn" is one complete round of user→LLM→tools→result. The turn loop is the heart of the agent:

```
handle_deepseek_turn()
  │
  ├─ 1. PRE-REQUEST PHASE
  │    ├─ Capacity checkpoint (token budget check)
  │    ├─ Context compaction if over threshold
  │    ├─ LSP diagnostics injection
  │    └─ Build MessageRequest
  │
  ├─ 2. STREAMING PHASE
  │    ├─ POST to DeepSeek API with stream: true
  │    ├─ Process SSE events:
  │    │   ├─ Text deltas → emit Status events
  │    │   ├─ Thinking deltas → emit thinking display
  │    │   ├─ ToolUse blocks → collect into ToolUseState
  │    │   └─ Message complete → finalize
  │    ├─ Stream guardrails:
  │    │   ├─ 10 MB content limit
  │    │   ├─ 30-minute wall-clock limit
  │    │   ├─ 5 consecutive error tolerance
  │    │   └─ 2 transparent stream retries
  │    └─ Fake tool call detection (scrub forged markers)
  │
  ├─ 3. TOOL EXECUTION PHASE
  │    ├─ Plan tool execution batch
  │    │   ├─ Classify: parallel-safe (read-only) vs serial
  │    │   ├─ Check approval requirements per tool
  │    │   └─ Build ToolExecutionPlan for each tool
  │    ├─ Execute batch:
  │    │   ├─ Parallel: spawn join tasks for safe tools
  │    │   └─ Serial: execute one at a time with approval
  │    ├─ For each tool:
  │    │   ├─ Pre-execution hooks
  │    │   ├─ Approval gate (if required)
  │    │   ├─ Sandbox preparation
  │    │   ├─ Execute tool
  │    │   ├─ Post-execution hooks
  │    │   └─ LSP post-edit diagnostics (for file writes)
  │    └─ Collect results as ToolResult messages
  │
  ├─ 4. POST-TURN PHASE
  │    ├─ Workspace snapshot (if enabled)
  │    ├─ Cycle advancement
  │    ├─ Usage/cost tracking
  │    └─ Emit TurnComplete event
  │
  └─ 5. LOOP DECISION
       ├─ If tool calls produced: append results, goto step 1 (new API call)
       ├─ If no tool calls: turn complete, await next user message
       └─ Loop guard: prevent infinite identical tool call loops
```

### 3.3 Tool Execution Planning

The dispatch system (`crates/tui/src/core/engine/dispatch.rs`) plans execution:

```rust
pub struct ToolExecutionPlan {
    index: usize,
    id: String,
    name: String,
    input: serde_json::Value,
    caller: Option<ToolCaller>,
    interactive: bool,
    approval_required: bool,
    supports_parallel: bool,
    read_only: bool,
}

pub enum ToolExecutionBatch {
    Parallel(Vec<ToolExecutionPlan>),
    Serial(Box<ToolExecutionPlan>),
}
```

Tools are classified by capability: `ReadOnly`, `Write`, `Sandboxable`, `SideEffect`. Read-only tools that are parallel-safe can be batched into a `Parallel` execution. Write tools and tools with side effects run serially.

---

## 4. System Prompt Assembly

The system prompt is assembled in layers, designed for DeepSeek's KV prefix cache stability:

```
┌──────────────────────────────────────────────┐
│  STATIC LAYERS (byte-stable for cache hits)  │
│                                              │
│  1. Base prompt (prompts/base.md)            │
│     Core identity + tool-use rules           │
│                                              │
│  2. Personality overlay                      │
│     CALM_PERSONALITY or PLAYFUL_PERSONALITY  │
│                                              │
│  3. Mode delta                               │
│     AGENT_MODE / PLAN_MODE / YOLO_MODE       │
│                                              │
│  4. Approval policy overlay                  │
│     AUTO_APPROVAL / SUGGEST_APPROVAL / ...   │
│                                              │
│  5. Skills block                             │
│     Active skill content                     │
│                                              │
│  6. Project context                          │
│     AGENTS.md / instructions content         │
│                                              │
│  7. Environment block                        │
│     Locale, version, platform, shell, PWD    │
│                                              │
├──────────────────────────────────────────────┤
│  VOLATILE LAYERS (change frequently)         │
│                                              │
│  8. User memory                              │
│     Persistent notes file                    │
│                                              │
│  9. Goals / handoff block                    │
│     Previous session relay artifact          │
│                                              │
│  10. Working set summary                     │
│      Recent file paths and activity          │
│                                              │
│  11. Locale reinforcement                    │
│      Preamble + closer in native script      │
│      (zh-Hans, ja, pt-BR)                    │
└──────────────────────────────────────────────┘
```

The separation between static and volatile layers is critical: the static prefix remains byte-identical across turns, maximizing DeepSeek V4 prefix cache hits. Only the volatile tail changes per request.

---

## 5. Context Window Management

### 5.1 Compaction

Triggered by token pressure only (no message-count trigger). The system:

1. Checks if token usage exceeds the compaction threshold (80% of model context window)
2. Enforces a **500K floor** — won't compact below this to preserve V4 prefix cache
3. Pins important messages: recent turns, working set paths, error messages, patches, tool call pairs
4. Prunes verbose duplicate tool results mechanically
5. Uses V4 Flash for cheap, fast summarization
6. Produces cache-aligned summaries that preserve message prefix

### 5.2 Seam Management (Layered Context)

An append-only architecture that avoids losing verbatim messages:

```
Soft seam levels at 192K, 384K, 576K tokens:
  ├─ Verbatim window: last 16 turns are never summarized
  ├─ <archived_context> blocks store compressed summaries
  ├─ Progressive summarization: older context → denser summaries
  └─ Recompaction: existing seams can be fused into denser blocks
```

This is more sophisticated than simple truncation — it preserves a verbatim recent window while compressing older context progressively.

### 5.3 Capacity Controller

Tracks token usage and enforces limits:

```rust
pub struct CapacityControllerConfig {
    // Token budgets and limits
    // Auto-compaction thresholds
    // Turn cost estimation
}
```

Per-turn capacity checkpoints prevent runaway context growth.

---

## 6. Tool System

### 6.1 Tool Registry

50+ tools organized by category. Each tool implements the `ToolSpec` trait:

```rust
pub trait ToolSpec {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn capabilities(&self) -> Vec<ToolCapability>;
    fn input_schema(&self) -> serde_json::Value;
    fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult;
}
```

**Tool categories:**

| Category | Tools | Notes |
|----------|-------|-------|
| File Ops | `read_file`, `write_file`, `edit_file`, `list_dir` | Edit uses search/replace with diff preview |
| Patching | `apply_patch` | Unified diff format, conflict detection |
| Shell | `exec_shell`, `shell_output` | Command execution with sandboxing |
| Git | `git`, `git_history`, `github` | Version control operations |
| Search | `file_search`, `web_search`, `fetch_url` | Local and web search |
| Development | `diagnostics`, `test_runner`, `review` | LSP integration, test execution |
| Automation | `automation`, `plan`, `tasks` | Task management and planning |
| MCP | `list_mcp_*`, `read_mcp_*` | Model Context Protocol tools |
| Sub-agents | `spawn_agent` | Recursive agent spawning |
| RLM | `rlm_query`, `sub_query_batch` | Recursive Language Model / REPL |
| Memory | `remember` | Persistent note updates |

### 6.2 Tool Execution Context

```rust
pub struct ToolContext {
    workspace: PathBuf,
    shell_manager: SharedShellManager,
    sandbox_policy: SandboxPolicy,
    working_set: Arc<Mutex<WorkingSet>>,
    // ... additional context
}
```

### 6.3 File Edit Protocol

Unlike simple text-marker systems, DeepSeek-TUI uses structured tool calls through the API's `tool_use` content blocks:

```
LLM response contains tool_use block:
  { "name": "edit_file", "input": {
      "path": "src/main.rs",
      "old_string": "fn main() { ... }",
      "new_string": "fn main() { ... updated ... }"
  }}

→ Tool dispatch extracts the tool call
→ Sandbox validates the path
→ Apply search/replace edit
→ Generate diff preview
→ Run LSP post-edit diagnostics
→ Return result to LLM
```

---

## 7. Permission & Approval System

### 7.1 Three Modes

| Mode | Shell | File Write | Approval | Toggle |
|------|-------|-----------|----------|--------|
| **Plan** | No | No | N/A | Shift+Tab |
| **Agent** | Yes | Yes | Per-action approval | Shift+Tab |
| **YOLO** | Yes | Yes | Auto-approve all | Shift+Tab |

### 7.2 Risk Classification

Every tool call is classified before execution:

```rust
pub enum ToolCategory {
    Safe,       // read_file, list_dir — no approval needed
    FileWrite,  // write_file, edit_file — approval in Agent mode
    Shell,      // exec_shell — approval in Agent mode
    Network,    // web_search, fetch_url — policy-based
    McpRead,    // MCP read operations
    McpAction,  // MCP write operations
    Unknown,    // Default: require approval
}

pub enum RiskLevel {
    Benign,       // Single-key approval (Y/N)
    Destructive,  // Two-key confirmation required
}
```

### 7.3 Approval Flow

```
Tool call received
  ├─ Classify risk via classify_risk()
  ├─ Build impact summary via build_impact_summary()
  ├─ If Auto mode (YOLO): skip approval, execute directly
  ├─ If Suggest mode (Agent):
  │    ├─ Benign: show "Y to approve, N to deny"
  │    └─ Destructive: show "YY to approve, N to deny" (staged)
  ├─ If Never mode: deny all tools requiring approval
  └─ Decision: Approved | ApprovedForSession | Denied | Abort
```

Session-level approval caching means approving a tool once approves it for the rest of the session.

### 7.4 Execution Policy Engine

Hierarchical rulesets with three layers:

```rust
pub enum RulesetLayer {
    BuiltinDefault = 0,  // Hardcoded safe defaults
    Agent = 1,           // Agent-defined rules
    User = 2,            // User overrides (highest priority)
}
```

Rules use **arity-aware prefix matching** for allow rules and simple prefix matching for deny rules. Deny always wins over allow.

---

## 8. Sandbox Security

### 8.1 Platform-Specific Sandboxing

| Platform | Mechanism | Notes |
|----------|-----------|-------|
| macOS | Seatbelt (`sandbox-exec`) | Mandatory access control profiles |
| Linux | Landlock (kernel 5.13+) | Filesystem access control rules |
| Windows | Job Objects | Process containment (planned) |

### 8.2 Sandbox Policies

```rust
pub enum SandboxPolicy {
    DangerFullAccess,     // No restrictions (dangerous)
    WorkspaceWrite { .. },  // Workspace-scoped access with optional network
    ReadOnly,             // Read-only filesystem access
    ExternalSandbox,      // External sandbox management
}
```

### 8.3 Execution Flow

```
CommandSpec::shell("cargo test")
  → with_policy(WorkspaceWrite)
  → SandboxManager::prepare()
     ├─ Platform-specific sandbox setup
     ├─ Path boundary enforcement
     └─ Network policy per-domain
  → Execute with timeout
  → Detect denial patterns
  → Parent-death signaling for cleanup
```

---

## 9. Streaming & Communication

### 9.1 SSE Streaming

The LLM client streams responses via Server-Sent Events:

```
POST /v1/chat/completions { stream: true }
  → async_stream of SSE chunks
  → Parse each chunk:
     ├─ ContentBlock::Text → emit text delta
     ├─ ContentBlock::Thinking → emit thinking delta
     ├─ ContentBlock::ToolUse → accumulate tool call input
     └─ Message complete → finalize
  → Backpressure via bytes_stream consumer
  → Cancellation via CancellationToken
```

### 9.2 Stream Guardrails

| Guardrail | Value | Purpose |
|-----------|-------|---------|
| `STREAM_MAX_CONTENT_BYTES` | 10 MB | Prevent memory exhaustion |
| `STREAM_MAX_DURATION_SECS` | 30 min | Wall-clock timeout |
| `MAX_STREAM_ERRORS_BEFORE_FAIL` | 5 | Error tolerance |
| `MAX_TRANSPARENT_STREAM_RETRIES` | 2 | Auto-retry on early failures |

### 9.3 Fake Tool Call Detection

The system scrubs forged tool-call markers from text output to prevent prompt injection:

- Markers detected: `[TOOL_CALL]`, `<deepseek:tool_call`, etc.
- `filter_tool_call_delta()` removes these from text streams

### 9.4 LLM Error Handling

```rust
pub enum LlmError {
    RateLimited { message, retry_after },
    ServerError { status, message },
    NetworkError(String),
    Timeout(Duration),
    AuthenticationError(String),
    InvalidRequest { status, message },
    ModelError(String),
    ContentPolicyError(String),
    ParseError(String),
    ContextLengthError(String),
    Other(String),
}
```

Retry logic uses exponential backoff with jitter for transient errors (rate limits, server errors, network issues).

---

## 10. Sub-Agent System

### 10.1 Agent Taxonomy

```rust
pub enum SubAgentType {
    General,      // Full tool access
    Explore,      // Read-only exploration
    Plan,         // Planning and analysis
    Review,       // Code review
    Implementer,  // Focused implementation
    Verifier,     // Test execution and validation
    Custom,       // Custom tool access
}
```

### 10.2 Sub-Agent Lifecycle

```
Spawn sub-agent with objective + role
  ├─ Filter tools by role (e.g., Explore → read-only tools only)
  ├─ Launch as non-blocking background task
  ├─ Monitor via mailbox system
  ├─ Structured output format:
  │    SUMMARY, CHANGES, EVIDENCE, RISKS, BLOCKERS
  ├─ Concurrency cap: default 10, max 20
  └─ Depth limit: max_spawn_depth prevents infinite recursion
```

Sub-agents have independent context windows and tool permissions. Results are collected and returned to the parent agent.

---

## 11. Session Persistence & Recovery

### 11.1 Session Structure

```rust
pub struct SavedSession {
    schema_version: u32,        // For migration
    metadata: SessionMetadata,
    messages: Vec<Message>,
    system_prompt: SystemPrompt,
    context_references: Vec<String>,
    artifacts: Vec<String>,
}

pub struct SessionMetadata {
    id: Uuid,
    title: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    message_count: usize,
    token_usage: TokenUsage,
    model: String,
    workspace: PathBuf,
    cost_snapshot: CostSnapshot,  // USD/CNY
}
```

### 11.2 Persistence

- SQLite database for threads, messages, checkpoints, jobs
- Atomic writes (write to temp, rename) for crash safety
- Maximum 50 sessions, 500 messages per session
- Session pruning of old data

### 11.3 Crash Recovery

- Checkpoint snapshots capture state at key points
- On restart, the system detects incomplete sessions
- Session replay from last checkpoint

---

## 12. Workspace Snapshot System

### 12.1 Side Git Repository

An independent git repo at `~/.deepseek/snapshots/<hash>/.git` captures workspace state:

- Uses `--git-dir` and `--work-tree` — never touches the user's `.git`
- Pre-turn snapshot before tool execution
- Post-turn snapshot after completion
- Non-fatal: snapshot failures never block TUI operation

### 12.2 Retention

- Default 7-day retention
- 50-snapshot cap per workspace
- Background pruning of stale snapshots
- Git's content-addressed storage deduplicates files

### 12.3 Restore

- `/restore N` command rolls back to snapshot N
- `revert_turn` undoes the last turn's changes
- Independent of user's git history

---

## 13. Working Set & Project Context

### 13.1 Working Set

Tracks which files the agent is actively working with:

```
Observe user messages and tool calls
  → Extract and normalize path candidates
  → Update entries with touch counts and recency
  → Frecency-based ranking (frequency + recency)
  → Prune to max_entries limit
  → Generate prompt summaries for context injection
```

### 13.2 Project Context Pack

Auto-generated from the workspace:

```
Project name + directory structure
  + README excerpt
  + Config files and source files
  + Entry counts
```

Used to give the LLM awareness of the project layout without loading every file.

---

## 14. Auto Model Selection

When `--model auto` is specified:

```
resolve_auto_route_with_flash()
  → Send lightweight routing request to V4 Flash
  → Analyze prompt complexity, reasoning requirements, context length
  → Select optimal model + reasoning effort level
  → Fallback to default model on routing failure
```

Reasoning effort levels: `Off`, `Low`, `Medium`, `High`, `Auto`, `Max`.

---

## 15. RLM — Recursive Language Model

A persistent Python REPL integration for code execution:

```
RlmSession (Python kernel + context metadata)
  ├─ Persistent sessions with SHA256 context hashing
  ├─ Sub-query support: nested RLM calls with depth limits
  ├─ sub_query_batch: parallel cheap child calls
  ├─ var_handle: output reference system
  ├─ Timeout handling: per-query (default 120s)
  └─ Recursion limit: sub_rlm_max_depth (default 1)
```

Configuration options: `OutputFeedback` (Full vs Metadata), `share_session` for reuse.

---

## 16. Hooks System

Lifecycle hooks for external integration:

```rust
pub enum HookEvent {
    ResponseStart / ResponseDelta / ResponseEnd,
    ToolLifecycle (pre/post execution),
    JobLifecycle (progress tracking),
    ApprovalLifecycle (approval phases),
}
```

Three sink implementations:
- **StdoutHookSink** — Console output
- **JsonlHookSink** — File-based JSONL logging with timestamps
- **WebhookHookSink** — HTTP webhook delivery with retries

Errors in individual sinks don't stop others — fault isolation.

---

## 17. LSP Integration

Post-edit diagnostics injection for type checking and linting:

```
File write tool completes
  → LSP post-edit hook fires
  → Language-specific server (rust-analyzer, pyright, etc.)
  → Collect diagnostics
  → Inject into tool result
  → Non-blocking: LSP failure doesn't block tool result
```

---

## 18. Complete Workflow — End to End

Here is the full path from a user typing a message to the agent delivering results:

```
┌─────────────────────────────────────────────────────────────────────┐
│                     INITIALIZATION                                   │
│                                                                     │
│  CLI args parsed                                                    │
│    → Config loaded (project → user → defaults)                      │
│    → API key resolved (keyring → env → config)                      │
│    → Skills loaded from .agents/skills, skills/, .deepseek/skills/  │
│    → MCP servers started (stdio transport, capability exchange)     │
│    → Project context loaded (AGENTS.md / instructions)              │
│    → Previous session restored from SQLite (if --resume)            │
│    → Engine spawned as background task                              │
│    → TUI rendered (ratatui alternate screen)                        │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     USER INPUT                                       │
│                                                                     │
│  User types message in composer                                     │
│    → Slash command? → /skill dispatch or built-in handling           │
│    → @mention? → attach file/context                                │
│    → Free text → construct Op::SendMessage                          │
│    → Send via mpsc channel to Engine                                │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     TURN EXECUTION                                   │
│                                                                     │
│  Engine receives Op::SendMessage                                    │
│    → Emit TurnStarted event → UI shows "thinking..."                │
│    → System prompt assembly:                                        │
│      base + personality + mode + approval + skills                  │
│      + project context + environment + memory + working set         │
│    → Capacity checkpoint (token budget)                             │
│    → Context compaction if needed (seam management)                 │
│    → Build MessageRequest with tools + system prompt + messages     │
│    → POST to LLM API (stream: true)                                 │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     STREAMING RESPONSE                               │
│                                                                     │
│  SSE stream produces content blocks:                                │
│    → Text deltas → emit Status events → UI renders text             │
│    → Thinking deltas → emit thinking display                        │
│    → ToolUse blocks → accumulate input buffers                      │
│    → Message complete → finalize stream                             │
│                                                                     │
│  Guardrails: 10MB limit, 30min timeout, 5-error tolerance           │
│  Fake tool call detection and scrubbing                             │
│  Transparent retry on early stream failures (up to 2)               │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     TOOL EXECUTION                                   │
│                                                                     │
│  If response contains tool_use blocks:                              │
│    → Plan execution batch:                                          │
│      classify each tool (read-only vs write vs side-effect)         │
│      → parallel-safe tools batched together                         │
│      → write/side-effect tools run serially                         │
│                                                                     │
│  For each tool:                                                     │
│    → Pre-execution hooks fire                                       │
│    → Risk classification (Safe / FileWrite / Shell / Network)       │
│    → Approval gate:                                                 │
│      YOLO mode: auto-approve                                        │
│      Agent mode: prompt user (Y/N, staged YY for destructive)       │
│      Plan mode: deny all writes                                     │
│    → Sandbox preparation (Seatbelt/Landlock)                        │
│    → Execute tool                                                   │
│    → Post-execution hooks fire                                      │
│    → LSP post-edit diagnostics (for file writes)                    │
│    → ToolResult collected                                           │
│                                                                     │
│  Loop guard: detect and prevent infinite identical tool calls        │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     RESULT DELIVERY                                  │
│                                                                     │
│  Tool results appended to message history                           │
│    → Emit ToolCallResult events → UI shows tool output              │
│    → If more tool calls needed: loop back to TURN EXECUTION         │
│    → If LLM produces final text (no tools): turn complete           │
│                                                                     │
│  Post-turn:                                                         │
│    → Workspace snapshot (side-git commit)                           │
│    → Usage/cost tracking updated                                    │
│    → Session auto-saved to SQLite                                   │
│    → Emit TurnComplete event → UI shows final state                 │
│    → Working set updated with touched paths                         │
│    → Engine awaits next Op from UI                                  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 19. Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Background engine task + mpsc channels | Keeps UI responsive during API calls and tool execution |
| Layered system prompt with static/volatile split | Maximizes DeepSeek V4 prefix cache hit rate |
| Platform-specific sandboxing | Real OS-level isolation, not just path validation |
| Seam-based compaction (not truncation) | Preserves verbatim recent context while compressing older turns |
| Side-git snapshots | Crash recovery and undo without touching user's repository |
| Structured tool_use blocks (not text markers) | Reliable parsing regardless of model quality |
| Parallel tool execution | Read-only tools run concurrently for faster turns |
| Two-stage approval for destructive ops | Prevents accidental approval of dangerous commands |
| Non-fatal snapshot and LSP | Failures in auxiliary systems never block the main loop |
| SQLite state store | Durable persistence with query capability for session management |

---

## 20. Comparison with LitePilot-TUI

| Aspect | LitePilot-TUI | DeepSeek-TUI |
|--------|--------------|--------------|
| Architecture | Single crate, ~20 modules | 17-crate workspace |
| Agent loop | Sync main loop + background threads | Async engine + mpsc channels |
| LLM integration | Ollama local models only | Multi-provider (DeepSeek, OpenAI, Ollama, etc.) |
| Tool protocol | Text markers (`### FILE:` + `### ACTION:`) | Structured `tool_use` API blocks |
| Context management | Token budget truncation | Seam-based layered compaction |
| Sandboxing | Path validation + command allowlist | OS-level (Seatbelt/Landlock) |
| Session storage | JSON files | SQLite database |
| Sub-agents | None | Typed sub-agents with depth limits |
| Streaming | SSE via async_stream | SSE with guardrails and transparent retry |
| Skills | Markdown files with frontmatter | Same, plus GitHub install and auto-loading |
| MCP support | None | Full MCP client + stdio server |
| LSP integration | None | Post-edit diagnostics injection |
| Snapshots | None | Side-git pre/post-turn snapshots |
| Approval | 3 coarse modes (Plan/Edit/Auto) | Risk-classified with staged confirmation |
| Retry | Response quality validation | API error retry with exponential backoff |
| Prompt assembly | Single template per model size | Layered with cache-stable prefix |
