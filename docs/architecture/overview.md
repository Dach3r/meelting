# Architecture

> **Status:** Phases 0–2 complete; Phase 3 with rooms, messages and agent runs (`AgentRun`) built. Today there is the Tauri 2 scaffold (React 19 + TypeScript + Vite + Tailwind v4), SQLite with `profile`, `workspaces`, `teams`, `agents`, `canvas_positions`, `rooms`, `room_agents` and `messages`, the first launch, the shell and workspace switching, teams, agents and the org chart, rooms with participants and messages, `agent_runs` and the execution engine with adapters for Anthropic, OpenAI and Chat Completions (Groq, Ollama, LM Studio), the `keychain` module with the `save_secret`, `secret_last_four` and `delete_secret` commands, and the `run_in_worktree` spike (git worktree + PTY with live output). The folder layout is in [project-structure.md](project-structure.md). The rest of this document describes the **target** architecture; each section marks what already exists.
>
> This product replaces the Rails SaaS (repository `meelting`), frozen as a design reference: domain model, states, errors, copy, templates and specs as test cases.

## 1. Context and decision

**Need:** agents must complete real code tasks. Each task needs its own `git worktree`, to run commands, start Docker containers and use local AI models.

**Requirements**
- **Hybrid online/offline operation:** supports **local models** (Ollama, LM Studio, llama.cpp) and cloud services, combinable and chosen by the user with no mandatory preference. Local things (data, history and agents with running local models) work offline; whatever depends on the cloud reports that it is unavailable and is never silently replaced by another provider ([decisions.md](../product/decisions.md) D6).
- Runs on **macOS, Windows and Linux**.
- Fast startup and low memory.
- **No remote API or account**: everything on the user's machine.

**Alternatives evaluated**

| Option | Result |
|---|---|
| Rails on a server + local runner | Discarded: does not work offline and leaves the brain outside the machine |
| Rails packaged in Tauri/Electron (sidecar) | Discarded: shipping Ruby and native gems on three OSes, migrating to SQLite and managing processes, gaining nothing without a server |
| Tauri client + remote Rails API | Discarded: without a remote server it adds nothing; adds sync and two codebases |
| **New desktop app, no server of its own (local data; local or cloud models)** | **Chosen** |

## 2. Architecture

```
┌────────────────────────── Tauri 2 application ─────────────────────────┐
│  UI (webview)                       Core (Rust)                         │
│  React + TypeScript           ←IPC→ Tauri commands and events           │
│  Tailwind (Grafito tokens)          ├─ SQLite repositories (sqlx)       │
│  State: stores + events             ├─ Execution engine (AgentRun)      │
│                                     ├─ LLM providers (reqwest + SSE)    │
│                                     ├─ Tools (fs, git, shell)           │
│                                     ├─ Terminal (portable PTY)          │
│                                     ├─ Docker (CLI/bollard, optional)   │
│                                     └─ Secrets (OS keychain)            │
└────────────────────────────────────────────────────────────────────────┘
        │ local HTTP            │ child processes          │ Docker socket
   Ollama / LM Studio       git, shell, PTY             Docker Engine
```

### Stack

| Layer | Choice | Status |
|---|---|---|
| Shell | Tauri 2 | ✅ scaffold |
| Core | Rust (edition 2024) | ✅ scaffold |
| UI | React 19 + TypeScript + Vite; Tailwind v4 with the Grafito tokens | React/Vite/Tailwind ✅; own `es`/`en` i18n ✅ |
| Window | `tauri-plugin-window-state`: remembers size, position and maximized state across launches; no permissions for the UI | ✅ |
| Secrets | crate `keyring` 3 (Keychain, Credential Manager, Secret Service) | ✅ spike |
| Data | SQLite with `sqlx`, WAL mode, `sqlx migrate` migrations | ✅ `profile`, `workspaces`, `teams`, `agents`, `canvas_positions`, `rooms`, `room_agents` and `messages`; the rest of the tables with their phase |
| HTTP/streaming | `reqwest` + SSE (`eventsource-stream`) | ✅ adapters for the three protocols |
| Terminal | `portable-pty` (ConPTY on Windows) | ✅ spike |
| Git | `git` CLI via `tokio::process` | ✅ spike |
| Docker | `docker` CLI via `tokio::process` (or `bollard`); optional | pending |
| Tests | `cargo test`, Vitest, WebDriver (`tauri-driver`) | `cargo test` and Vitest ✅; WebDriver pending |

**Git and Docker are invoked as processes, not with libraries:** `git2` (libgit2) does not fully support `worktree` or the user's hooks, and the CLI is what the user expects.

## 3. Core layers

- **Tauri commands** (`#[tauri::command]`, in `commands/`): thin. They validate arguments, call a service and return the result. No business rules or SQL.
- **Services** (`services/`): business actions (verbs): hire agent, create room, send instruction, create task. They receive the current workspace and validate cross-table integrity.
- **Repositories**: SQLite access; `AgentRun` transitions are atomic functions within a transaction.
- **Catalogs**: read-only collections embedded in the app (`agent_templates/*.yml`, `llm.yml`), read with `serde_yaml`. A catalog never orchestrates: the action that uses it is a service.
- **OS integration** (`system/`): child processes (`git`), PTY and keychain; no business rules.
- **Pure domain** (models, `BuildPrompt`, provider normalization, path jail) separated from the OS integration code, so it can be tested without I/O.
- **Events** (`emit`) from the core to the UI: `message:created`, `agent_run:status` (✅, both after the transaction commits) and `agent_run:token` (✅, batched every ≤ 50 ms, with `attempt` and `seq`); `tool_call:output` planned. The events proposed for meetings and voice (plan, availability, capture, transcription, playback) are in [conversation-engine.md §11](conversation-engine.md#11-proposed-ipc-contracts-names-not-final).
- **Agent runs** (✅ built according to the approved design): [agent-run.md](agent-run.md).
- **Conversational meetings and voice** (proposal): [conversation-engine.md](conversation-engine.md) and [voice-engine.md](voice-engine.md). Audio capture and playback would live in the core; `AgentRun` stays independent of audio.

The webview **gets no direct access to `fs` or `shell`**: Tauri capabilities are declared minimal, only the app's own commands.

Folder layout, responsibilities and where to put new code: [project-structure.md](project-structure.md).

## 4. Map from the Rails product

| Rails | Destination |
|---|---|
| `User`, Devise, account onboarding | Removed. A local `profile` (name, language) and first launch |
| `Workspace`, `Team`, `Agent`, `Room`, `RoomAgent`, `Message` | Kept |
| `AgentRun` and its state machine | Kept as an async task in Rust |
| `LlmCredential` (Postgres, encrypted) | OS keychain; the table only records which providers have a key |
| `AgentTemplate`, `LlmProvider`, `LlmModel` | Embedded files `agent_templates/*.yml` and `llm.yml` |
| `Llm::Complete` + adapters | `Provider` trait in Rust |
| `Llm::Error` (`Transient`/`Permanent`) | `LlmError` enum with the same `kind`s |
| Solid Queue, `AgentRunJob`, `ExpireStaleAgentRunsJob` | `tokio` tasks; at startup, orphaned `running` ones become `failed` |
| Solid Cable, Turbo Streams, `RoomChannel` | Tauri events |
| Controllers | Tauri commands |
| Rate limiting by IP/user | Removed; optional daily limits |
| `config/locales/{es,en}.yml` | Basis of the frontend i18n JSON |
| ViewComponent components (atomic design) | React components with the same hierarchy and names |
| Stimulus controllers | Components and hooks (see §8) |

## 5. Conventions

- Internal names in **English**, `snake_case` in tables and columns.
- Visible text translated, never written in the code. Spanish uses "correo electrónico", not "email".
- No code comments; code should explain itself.
- **No dead code:** every method, command, component or constant has a real use.
- No over-engineering: with two or three cases, a direct conditional.
- Validations mirrored in the database (unique indexes, foreign keys, `CHECK`).
- gitmoji + Conventional Commits, one logical change per commit.
- Linters: `clippy -D warnings`, `rustfmt --check`, ESLint, `tsc`.
- Do not create migrations or change columns without presenting the proposal and getting approval.

## Data model

> **Proposal.** Presented for approval before creating any migration.

General rules:
- `TEXT` IDs with **UUID v7** generated in the application (sortable, ready for a future sync).
- `TEXT` ISO-8601 UTC dates (`created_at`, `updated_at`).
- `PRAGMA foreign_keys = ON`, `journal_mode = WAL`.

| Table | Columns | Notes |
|---|---|---|
| `profile` (one row) | `id` (`'singleton'`), `locale` (`es`/`en`), `theme` (`system`/`light`/`dark`, default `system`), `ui_scale` (`90`/`100`/`110`/`125`/`150`, default `100`), `current_workspace_id`, timestamps | Replaces `users`. First and last name are added when there is UI that asks for them |
| `workspaces` | `id`, `name` (1–80), timestamps | |
| `teams` | `id`, `workspace_id`, `name` (1–80), timestamps | Unique `(id, workspace_id)` for the composite foreign key of `agents` |
| `agents` | `id`, `workspace_id`, `team_id`, `template_key`, `name` (1–80), `role` (1–80), `description`, `instructions`, `provider`, `model`, timestamps | Composite FK `(team_id, workspace_id)` `ON DELETE RESTRICT`; `provider`/`model` not empty |
| `canvas_positions` | `workspace_id`, `node_id`, `x`, `y` | PK `(workspace_id, node_id)`; `node_id` is a team or agent; `x`,`y` are the offset from the org chart's automatic layout; deleted with the workspace and when the agent is deleted |
| `rooms` | `id`, `workspace_id`, `name` (1–80), `created_at`, `updated_at`, `finished_at` (NULL = call in progress; `CHECK (finished_at IS NULL OR finished_at >= created_at)`) | Unique `(id, workspace_id)`. `updated_at` is the last activity: equal to `created_at` on creation and updated with each message in the same transaction. Index `(workspace_id, updated_at DESC, id DESC)` for the list; finished rooms are listed last and the core treats them as nonexistent (`room_not_found`) |
| `room_agents` | `room_id`, `agent_id`, `workspace_id`, `created_at`, `left_at` (NULL = present) | PK `(room_id, agent_id)`, no `id` or `updated_at`. `left_at >= created_at`; leaving the room sets `left_at` and joining again sets it to NULL (D24). Composite FKs `(room_id, workspace_id) → rooms` and `(agent_id, workspace_id) → agents`, both `ON DELETE CASCADE`; the second uses the unique index `agents (id, workspace_id)` |
| `messages` | `id`, `room_id`, `agent_id` (NULL = user), `body`, `created_at` | Immutable, no `updated_at`. `body` not empty; maximum 4000 characters only if it is from the user (`agent_id IS NULL`). `room_id → rooms` `ON DELETE CASCADE`; `(room_id, agent_id) → room_agents` `ON DELETE NO ACTION`: an agent only writes in rooms where it participates, an agent with messages cannot be deleted, and removing it from the room leaves its participation with `left_at` instead of deleting it. When the room is deleted, the check at the end of the statement allows deleting participations and messages together |
| `agent_runs` | ✅ Migrations approved and applied ([agent-run.md §2](agent-run.md#2-final-sql-schema)): `id`, `room_id`, `agent_id`, `request_message_id`, `response_message_id`, `client_request_id`, `status`, `error_kind`, `attempts`, `provider`, `model`, `input_tokens`, `output_tokens`, `created_at`, `started_at`, `finished_at` | Unique: `(room_id, client_request_id)`, `request_message_id`, `response_message_id` and one active run per `(room_id, agent_id)` (partial index). Composite FKs to `messages` so that request and response belong to the same room and the response to the same agent; they require two unique indexes on `messages` (✅). Does not port from Rails `user_id`, `updated_at`, `finish_reason`, `response_model`, `cached_input_tokens` or `provider_request_id` |
| `llm_providers_configured` | `provider`, `base_url`, `updated_at` | Planned for the "AI keys" screen; `AgentRun` does not need it (it queries the keychain when running) |
| `tasks` | `id`, `workspace_id`, `room_id`, `agent_id`, `title`, `repo_path`, `base_branch`, `branch`, `worktree_path`, `status`, `created_at`, `updated_at`, `finished_at` | Phase 4. `status`: `preparing`, `ready`, `running`, `waiting_approval`, `done`, `failed`, `discarded` |
| `tool_calls` | `id`, `agent_run_id`, `task_id`, `tool`, `arguments` (JSON), `status`, `output`, `exit_code`, `created_at`, `finished_at` | Phase 5 |
| `approvals` | `id`, `tool_call_id`, `decision` (`pending`/`approved`/`denied`), `decided_at`, `remember` (`never`/`task`/`workspace`) | Phase 5 |
| `settings` | `key`, `value` | Daily limits, Docker path, preferred shell |

**Cross-table integrity:** workspace isolation (a team, agent or participant must belong to the same workspace) is validated in the services layer and tested; where possible it is reinforced with composite foreign keys `(id, workspace_id)`.

## 6. Agent engine and providers

Phase 3 providers (approved decision D9): the five in the real catalog `src-tauri/src/catalogs/llm.yml`, with three protocols. Approved and built design ([agent-run.md](agent-run.md)): the adapter is chosen with a `match` on the protocol declared in the catalog, with no trait while there are three cases.

| Protocol | Providers | Notes |
|---|---|---|
| Messages API, SSE | `anthropic` | `system`, `messages`, `max_tokens`; text = `text` blocks |
| Responses API, SSE | `openai` | `instructions` + `input`; `store: false`; reasoning tokens count toward `max_output_tokens` |
| Chat Completions, SSE | `groq` (cloud, with key), `ollama` and `lmstudio` (local, no key; models via `GET /v1/models`) | Instructions as `system`; longer timeout locally; output tokens parameter per the catalog. Replaces the `stream_completion` spike |

Rules:
- The key is only read from the keychain in the core; **it never reaches the UI, logs or events**. Prompts and reasoning are not logged.
- A run **never switches provider or model** if it fails.
- `max_retries = 0` in the HTTP client; only the engine retries.
- Errors do not include provider text: they are reduced to an `error_kind` by HTTP status and code.
- A response that is not valid JSON or SSE is `invalid_response`.
- Models and providers live **only** in `llm.yml`; the code names no model.

**Errors (`error_kind`):** transient `rate_limited`, `timeout`, `provider_unavailable`; permanent `provider_not_configured`, `model_unavailable`, `provider_auth_failed`, `provider_billing`, `provider_rejected`, `content_filtered`, `output_truncated`, `invalid_response`, `context_too_large`, plus `cancelled` and `expired`. HTTP: 401/403 → auth, 402 → billing, 404 → model unavailable, 408 → timeout, 429 → rate limit (honoring `retry-after`; `insufficient_quota` → billing), 5xx and 529 → unavailable, rest → rejected.

**`AgentRun` state machine:** `queued → running → succeeded | failed`. Transitions only through repository functions, atomic and state-guarded: `claim` (increments `attempts`), `requeue`, `succeed`, `fail`, `cancel`, `expire_active`. Up to 3 attempts with waits of 2 and 8 s. Idempotency through unique `(room_id, client_request_id)` and a partial index. When the app starts, every `queued` or `running` run becomes `failed` with `error_kind = expired` (✅). Cancellation uses `CancellationToken` and ends in `failed` with `error_kind = cancelled`. Partial text is accumulated in memory; only the final message is persisted.

**Prompt context (`BuildPrompt`):** agent instructions, room name (there is no goal column, D11) and the room messages before the instruction, up to an internal character cap and 100 messages, frozen when the run starts; others' messages are sent with their name. **Response length** (D10): no character limit; it is bounded by the catalog's output tokens and a truncated response fails as `output_truncated` without creating a message.

## 7. Tasks, worktrees and tools

Lifecycle: the user creates a task from a room (agent, repository, base branch) → `preparing` validates `git` → the branch `meelting/<slug>-<short id>` and the worktree in `<app data>/worktrees/<workspace>/<task>` are created with `git worktree add -b` → the agent works only inside `worktree_path` → the user reviews `git diff <base>...<branch>` and keeps the branch, discards it (`git worktree remove --force` + `git branch -D`) or opens the folder in their editor.

Rules: several tasks from the same repo run in parallel in different worktrees; default concurrency of 2 active tasks per workspace; a reconciler at startup compares `git worktree list` with `tasks` and marks orphans without deleting anything; **never** `push`, `merge` or modify the base branch.

| Tool | Effect | Approval |
|---|---|---|
| `read_file`, `list_dir`, `search` | Read inside the worktree | No |
| `write_file`, `edit_file` | Write inside the worktree | No (reversible with git) |
| `run_command` | Command with cwd in the worktree | **Yes**, unless allowlisted |
| `git_status`, `git_diff`, `git_commit` | Git scoped to the worktree | `git_commit` no; no `push` at all |

**Security (main product risk):**
- **Path jail:** every path is canonicalized and must stay inside the worktree; `..`, escaping symlinks and external absolute paths (including UNC and drive letters) are rejected.
- **Human approval** of `run_command` with the exact command.
- **Sanitized environment:** the child process does not inherit `*_API_KEY`, tokens or `SSH_AUTH_SOCK`.
- **Timeout and output limit** per command; the process tree is killed.
- **Optional Docker** as strong isolation (worktree mounted, no network, non-root user); without Docker the app warns that execution is not isolated.
- Minimal Tauri capabilities.

Live output is emitted as `tool_call:output` with a size limit and shown in a panel per task.

**Cross-platform compatibility**

| Topic | macOS | Linux | Windows |
|---|---|---|---|
| `run_command` shell | `zsh`/`bash` | `bash`/`sh` | `PowerShell` (configurable) |
| Keychain | Keychain | Secret Service (libsecret) | Credential Manager |
| Docker | Docker Desktop | Docker Engine | Docker Desktop (WSL2) |
| PTY | PTY | PTY | ConPTY |
| Webview | WKWebView | WebKitGTK | WebView2 |
| Git on Windows | — | — | `core.longpaths`, respect `core.autocrlf` |

The Linux webview is the least predictable: it is tested in CI from Phase 0.

## 8. Interface

**Grafito design:** tokens `ink`, `surface`, `shell`, `rail`, `raised`, `line`, `primary`, `sand` in Tailwind v4's `@theme` (values copied from `app/assets/tailwind/application.css` in the Rails repository). Icons: `lucide`.

**Atomic design** in `src/components/<level>/<Name>`, with the same names as in Rails:
- **atoms:** `AgentAvatar`, `Button`, `Checkbox`, `Input`, `Label`, `Link`, `Logo`, `Select`, `StatusPill`.
- **molecules:** `AgentPicker`, `AgentRow`, `AgentRunStatus`, `AgentTile`, `Alert`, `CheckboxField`, `ConfirmDialog`, `ErrorSummary`, `FieldGroup`, `FormField`, `IdentityPill`, `InstructionsDocument`, `IslandMenu`, `LinkList`, `LocaleSwitcher`, `Message`, `MessageComposer`, `ModelSelect`, `NavItem`, `ParticipantTile`, `RoomCard`, `SettingsRow`, `TeamNode`, `Toast`, `WorkspaceSwitcher`.
- **organisms:** `Rail`, `RoomChat`, `TeamTree`, `ToastRegion`, `Topbar`.
- **templates:** `Workspace`, `Settings`.
- **new:** `TaskCard`, `TaskPanel`, `TerminalPanel`, `ApprovalDialog`, `ToolCallRow`, `DiffViewer`.
- Not migrated: Devise auth (`AuthInput`, `AuthField`, `AuthCard`, `Auth` template); `UserMenu` is simplified.

| Stimulus (Rails) | New frontend |
|---|---|
| `canvas` (draggable org chart, SVG lines, saved with a 500 ms debounce) | `TeamTree` with React Flow or custom SVG; positions via a Tauri command |
| `composer`, `scroll_bottom`, `local_time` | `MessageComposer` and `RoomChat` hooks |
| `agent_picker`, `agent_wizard`, `llm_model` | `AgentPicker`, hiring wizard, `ModelSelect` |
| `confirm`, `modal`, `drawer`, `disclosure`, `sidebar`, `toast`, `password_visibility` | Equivalent components |

Screens: teams (org chart), hire and edit agents, catalog, rooms (list, detail, new), AI keys, settings (profile, language, workspace, Docker, shell, local provider), tasks and task detail with terminal and diff. Rules: empty states, errors and confirmations always present; never show what is not built as available.

## 9. Internationalization

- Languages `es` and `en`; the code default is `en`; the chosen language is stored in `profile.locale` and changed from Settings. The theme (`system`, `light`, `dark`) is stored in `profile.theme`. The interface size (90–150 %) is stored in `profile.ui_scale` and applied as the `font-size` of `<html>`, so everything measured in `rem` scales.
- Starts from `config/locales/{es,en}.yml` in the Rails repository, filtering out Devise keys and those of removed features, converted to nested JSON.
- Provider error texts are resolved in the UI from `error_kind`; the core does not send them.
- Agent templates keep their YAML with `es` and `en` content.
- A test fails if a key exists in one language and not in the other.

## 10. Keys and privacy

- Keys in the keychain under the `meelting` service and the name `<workspace>/<provider>`. Never in SQLite, configuration files or logs.
- They are **verified before saving** with a minimal call to the provider.
- The UI only receives the "connected" status and the last 4 characters.
- No telemetry or network calls except to the configured providers; with only the local provider the app does not go out to the internet.
- Data lives in the app data directory (`~/Library/Application Support`, `%APPDATA%`, `~/.local/share`) and the user can export or delete it from Settings.

## 11. Tests

| Level | Tool | Covers |
|---|---|---|
| Rust core | `cargo test` | Repositories and `CHECK`, `AgentRun` machine, `BuildPrompt`, providers with a mock server (`wiremock`), path jail, tool loop |
| Provider contract | Shared suite | Same cases for the three protocols (success, tokens, each ending, each error) without real APIs |
| Git | Real temporary repos | Worktrees, orphans, paths with spaces and Unicode |
| Components | Vitest + Testing Library | Every atomic design component |
| E2E | `tauri-driver` in CI | Create workspace, hire agent, send instruction with a mock provider, approve a command, discard a task |
| Real providers | Optional, with an environment variable | Never in CI by default; includes Ollama |

CI (`.github/workflows/ci.yml`), three jobs in parallel: `lint` (Ubuntu: `rustfmt --check`, `clippy -D warnings`, `pnpm build` and `pnpm test`), `test` (`cargo test` on macOS, Windows and Ubuntu) and `bundle` (installers on the three OSes, only on `main` or manually, and prints the size of each installer). The Rails specs (`spec/services/llm`, `spec/models/agent_run_spec.rb`, `spec/catalogs`, `spec/requests`) are read as a list of cases to port: every acceptance criterion in [agent-execution.md](../product/agent-execution.md) must have an equivalent test. The coverage policy (100 % in the core domain, high and measured elsewhere) is pending a decision.

## 12. Packaging and distribution

- Installers: `.dmg`/`.app` (macOS Intel and Apple Silicon), `.msi`/`.exe` (Windows), `.AppImage` and `.deb` (Linux), generated by the Tauri bundler in GitHub Actions.
- **Signing:** macOS with Developer ID and notarization; Windows with a code signing certificate. Start the paperwork early.
- **Updates:** `tauri-plugin-updater` with signed artifacts (e.g. GitHub Releases); optional.
- Targets: installer < 30 MB, cold start < 2 s on average hardware.
- Local models are **not included**: the app detects Ollama/LM Studio and guides the installation.

## 13. Open decisions

| # | Decision | Recommendation |
|---|---|---|
| 1 | Tauri or Electron | Tauri; Electron if Phase 0 shows problems on Linux or Rust expertise is lacking |
| 2 | UI framework | React (already chosen in the scaffold; graph library for the org chart) |
| 3 | Data from the current Rails app | Do not migrate it, unless there is real data |
| 4 | Coverage policy | 100 % in the core domain, high elsewhere, measured in CI |
| 5 | Daily limits | Optional and disabled for local providers; after the first cut of `AgentRun` |
| 6 | Cancel a run | Include it (essential with slow local models) |
| 7 | Auto-update | Optional via signed GitHub Releases |
| 8 | Minimum OS support | macOS 12+, Windows 10+ (WebView2), Ubuntu 22.04+ |
| 9 | Task concurrency | 2 per workspace |
| 10 | Delete a workspace (Rails only allowed cancelling the account) | Allow it from Settings with confirmation; deletes teams, agents, rooms and keychain keys. Approval pending |

New decisions (meetings, voice, hybrid operation) are recorded in [decisions.md](../product/decisions.md).

## 14. Risks

| Risk | Mitigation |
|---|---|
| An agent runs a destructive command | Isolated worktree, path jail, mandatory approval, sanitized environment, optional Docker, never `push`/`merge` |
| WebKitGTK or WebView2 behave differently | CI matrix and E2E from Phase 0; Electron as plan B |
| Paths, shells and line endings on Windows | Git tests with spaces and Unicode, `core.longpaths`, configurable shell |
| Slow local models or with little context | Streaming, cancellation, configurable context cap |
| Code signing | Start the paperwork in Phase 1 |
| Rewriting the UI from scratch | Parity measured against the criteria in `docs/product/` and the Rails specs |
| Rust learning curve | Phase 0 as a trial; pure domain separated from the integration code |
| Scope creep toward integrations and collaboration | Keep the explicit exclusion in the roadmap |
