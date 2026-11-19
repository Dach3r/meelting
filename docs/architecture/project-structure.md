# Project structure

> **Status:** Phases 0–2 complete; rooms, messages, the `AgentRun` core (runs with streaming, retries and cancellation) and the "AI providers" screen of Phase 3 built. Only the modules listed here as "✅" exist. Whatever is marked "pending" is the **planned** location; the folder is not created until there is code that uses it (no dead code).

Layer rules and conventions: [overview.md](overview.md) §3 and §5.

## Overview

```
meelting-desktop/
├── CLAUDE.md              Project instructions (English)
├── LICENSE                FSL-1.1-MIT
├── README.md
├── docs/
│   ├── product/           Vision, roadmap, users, virtual office, rooms, agents, meetings and decision log
│   └── architecture/      overview.md, this document, AgentRun, conversation engine and voice engine (proposals)
├── .github/workflows/     CI on macOS, Windows and Ubuntu
├── index.html             Vite entry point
├── package.json           Frontend scripts and dependencies (pnpm)
├── src/                   Frontend (React + TypeScript)
└── src-tauri/             Core (Rust + Tauri 2)
```

## Rust core (`src-tauri/src`)

Dependencies always point downward: `commands → services → system`. No layer imports from a higher one.

```
src-tauri/src/
├── main.rs                Startup; only calls meelting_lib::run()
├── lib.rs                 Tauri builder, window state plugin, run expiry at startup, execution engine as state and command registration
├── commands/              Layer 1: Tauri commands (thin)
│   ├── provider_keys.rs   ✅ list_provider_keys, save_provider_key, check_provider_key, delete_provider_key (take only the provider; the core resolves the workspace)
│   ├── worktree.rs        ✅ run_in_worktree
│   ├── agent_runs.rs      ✅ send_intervention, resolve_recipient, check_agent_availability, cancel_agent_run and the engine's event emitter (agent_run:status, agent_run:token, message:created)
│   ├── rooms.rs           ✅ list_rooms, create_room, get_room (with runs, snapshots and agents who left), finish_room, add_room_agents, remove_room_agent
│   └── session.rs         ✅ load_session, complete_first_launch, create_workspace, switch_workspace, list_teams, create_team, create_agent, get_agent, update_agent, delete_agent, save_canvas_positions, reset_canvas_positions, list_agent_templates, list_agent_roles, list_llm_providers, detect_local_models, change_locale, change_theme, change_ui_scale, change_loading_enabled
├── services/              Layer 2: business actions (verbs)
│   ├── worktree_run.rs    ✅ creates a worktree and runs a command in it
│   ├── session_load.rs    ✅ loads the profile and the workspaces; defines SessionError
│   ├── engine_check.rs    ✅ validates a provider/model pair: cloud against `llm.yml`, local against the models installed in the running runtime (localhost); used by creating and editing agents
│   ├── first_launch.rs    ✅ creates the profile and the first workspace in one transaction
│   ├── workspace_create.rs ✅ creates a workspace and makes it current
│   ├── workspace_switch.rs ✅ changes the current workspace
│   ├── teams_list.rs      ✅ teams of the current workspace with their agents
│   ├── locale_change.rs   ✅ saves the profile language
│   ├── theme_change.rs    ✅ saves the profile theme
│   ├── ui_scale_change.rs ✅ saves the profile interface size
│   ├── loading_change.rs  ✅ saves whether the transition effect is on
│   ├── team_create.rs     ✅ creates a team and returns the updated list
│   ├── agent_create.rs    ✅ validates and creates an agent (custom or from a template) and returns the updated list
│   ├── agent_get.rs       ✅ full detail of an agent in the current workspace
│   ├── availability_check.rs ✅ availability of a room's agents: local based on the server and installed models (localhost), cloud based on the catalog, the keychain key and the last in-memory connection failure; no network calls to the cloud
│   ├── agent_update.rs    ✅ validates and edits an agent's name, role, description, instructions, team, provider and model (the pair is validated against the catalog only if it changes)
│   ├── agent_delete.rs    ✅ deletes an agent from the current workspace and returns the updated list; with messages or runs it fails with agent_has_activity
│   ├── rooms_list.rs      ✅ rooms of the current workspace by recent activity, with their agents and the last 3 messages of the first one
│   ├── recipient_resolve.rs ✅ resolve_recipient: recipient preview with the same rules as send_intervention, without saving anything
│   ├── room_create.rs     ✅ validates the name, deduplicates the agents and creates the room with its participants in one transaction
│   ├── room_get.rs        ✅ room of the current workspace with its present agents and those who left, its last 100 messages, its runs, the buffer snapshots (taken before reading the database) and the proposed recipient if still participating
│   ├── room_agents_add.rs ✅ invites agents to an existing room (deduplicates, validates the workspace, rejoins whoever had left) and returns present and departed agents
│   ├── room_agent_remove.rs ✅ removes an agent from the room (sets left_at) and cancels its active run in the same transaction
│   ├── room_finish.rs     ✅ finishes a room (`finished_at`), cancels its active runs and returns the updated list
│   ├── provider_keys_list.rs ✅ cloud providers of the catalog with the last 4 characters of the current workspace's key and the in-memory offline mark; no network
│   ├── provider_key_check.rs ✅ resolves a cloud provider and its keychain name, verifies a key with a minimal request through llm_call (updating the offline mark) and re-verifies the stored key without ever deleting it
│   ├── provider_key_save.rs ✅ validates the key, verifies it and only then saves it; a failed replacement keeps the previous key
│   ├── provider_key_delete.rs ✅ deletes the key of the current workspace (idempotent) and returns the updated list
│   ├── message_send.rs    ✅ saves a user message without a run and updates the room activity in one transaction (used by intervention_send)
│   ├── intervention_send.rs ✅ send_intervention: validates, deduplicates by client_request_id, resolves the recipient with domain/recipient inside the transaction and saves the user message (with the queued run if there is an agent) or returns the candidates to choose from without saving anything
│   ├── agent_run_engine.rs ✅ execution engine: pool, registry, catalog, limits, local (1) and cloud (4) semaphores and event emitter; launches each run as a tokio task
│   ├── agent_run_execute.rs ✅ a run's task: slot wait, claim, frozen context, adapter call, tokens batched every 50 ms, retries with the slot released, maximum duration, atomic success and failures
│   ├── agent_run_registry.rs ✅ in-memory registry of accepted runs (attempt text bounded by L1, sequence, frozen prompt and cancellation token), last agent that replied in each room (D19) and providers whose last connection failed
│   ├── agent_run_cancel.rs ✅ idempotent cancellation filtered by workspace
│   ├── agent_runs_expire.rs ✅ expires queued and running runs at startup
│   ├── canvas_positions_save.rs ✅ saves the org chart positions of teams and agents of the current workspace
│   ├── canvas_positions_reset.rs ✅ deletes the org chart positions of the current workspace and returns the updated list
│   ├── agent_roles_list.rs ✅ predefined roles from the catalog, grouped by area
│   ├── agent_templates_list.rs ✅ catalog templates in the profile language
│   ├── llm_providers_list.rs ✅ catalog providers and models
│   └── local_models_detect.rs ✅ queries the catalog's local providers and returns whether they are running and their models
├── system/                Layer 3: OS integration, no business rules
│   ├── git.rs             ✅ git via tokio::process (worktree add)
│   ├── terminal.rs        ✅ portable PTY and output reading
│   ├── llm_stream.rs      ✅ HTTP + SSE shared by the adapters: connection (marks unreachable if it could not connect), inactivity, HTTP errors by status and code, retry-after
│   ├── anthropic_messages.rs ✅ Anthropic Messages API adapter
│   ├── openai_responses.rs ✅ OpenAI Responses API adapter
│   ├── chat_completions.rs ✅ Chat Completions adapter (Groq, Ollama, LM Studio)
│   ├── llm_call.rs        ✅ picks the adapter for the catalog provider's protocol and streams the request; shared by the engine and key verification
│   ├── local_models.rs    ✅ lists the models of a local provider (GET /models)
│   └── keychain.rs        ✅ OS keychain (crate keyring)
├── repositories/          SQLite access (sqlx); migrations live in src-tauri/migrations
│   ├── database.rs        ✅ opens the pool (WAL, foreign keys) and applies migrations
│   ├── profile.rs         ✅ profile table queries
│   ├── workspaces.rs      ✅ workspaces table queries
│   ├── canvas_positions.rs ✅ canvas_positions table queries
│   ├── teams.rs           ✅ teams table queries
│   ├── agents.rs          ✅ agents table queries
│   ├── rooms.rs           ✅ rooms table queries
│   ├── room_agents.rs     ✅ room_agents table queries: join or rejoin, leave (left_at), present and departed
│   ├── messages.rs        ✅ messages table queries
│   └── agent_runs.rs      ✅ agent_runs queries and guarded transitions, and the schema integrity tests
├── domain/                Pure models and logic, no I/O
│   ├── agent_run.rs       ✅ states, error_kind, run, snapshot, injectable internal limits and retry policy
│   ├── recipient.rs       ✅ recipient resolution: selection (automatic, nobody, agent, confirmed), @ mentions by name or role ignoring case and accents, leading vocative, ambiguity, multiple agents and conflict
│   ├── prompt.rs          ✅ BuildPrompt: instructions, framing, labeled history (author and, in the user's questions, the agent they were addressed to) and budget; context_too_large if the minimum does not fit
│   ├── provider_error.rs  ✅ normalization of provider errors by HTTP status and code
│   ├── provider_key.rs    ✅ keychain name of a provider key (`<workspace_id>/<provider>`), key format validation and the verification result (verified, rejected, offline, provider_busy)
│   ├── local_provider.rs  ✅ detected local provider (key, running, models)
│   ├── locale.rs          ✅ es/en language
│   ├── theme.rs           ✅ system/light/dark theme
│   ├── ui_scale.rs        ✅ interface size (90–150 % presets)
│   ├── profile.rs         ✅ local profile
│   ├── room.rs            ✅ room, participant, message and their lists; body validation and agent deduplication
│   ├── session.rs         ✅ profile + workspaces received by the UI
│   ├── team.rs            ✅ team and agent (summary) received by the UI; AgentDraft and its validation
│   └── workspace.rs       ✅ workspace and name validation
└── catalogs/              Read-only collections embedded in the app
    ├── agent_roles.rs     ✅ loads the predefined agent roles (YAML) and validates that a role exists
    ├── agent_roles.yml    ✅ roles grouped by area
    ├── agent_templates.rs ✅ loads the agent templates (YAML es/en) and validates keys
    ├── agent_templates/   ✅ one template per predefined role (25 .yml, e.g. product_owner, qa_engineer, legal_advisor)
    ├── llm.rs             ✅ loads providers (protocol, base_url, output tokens parameter) and models with their output maximum, and validates that a provider/model pair exists; the UI only receives the names
    └── llm.yml            ✅ cloud and local providers with their models
```

### Responsibility of each layer

| Layer | What it does | What it does not do |
|---|---|---|
| `commands` | Receives arguments from the UI, resolves Tauri-specific things (`AppHandle`, app paths), calls a service, emits events and returns the result | Business rules, SQL, processes |
| `services` | Orchestrates a complete action and defines its serializable error for the UI | Know about Tauri (`AppHandle`, events): it receives *callbacks*, not emitters |
| `system` | Talks to the OS: child processes, PTY, keychain. Each module has its own error | Decide what to do or serialize errors toward the UI |
| `repositories` | SQLite queries and atomic transitions | Cross-table rules (they go in services) |
| `domain` | Pure logic testable without I/O (prompt, normalization, path jail) | Touch disk, network or processes |

### Core conventions

- **One error per module** with `thiserror`. `Serialize` is only implemented on the error that crosses to the UI (the service's or `keychain`'s), as a string.
- **Secrets only through services:** the webview never names a keychain entry. `commands/provider_keys.rs` takes a provider; the services check that it is a cloud provider of the catalog and build `<workspace_id>/<provider>` from the current workspace.
- **Errors to the UI as codes:** `SessionError` is serialized as a stable code (`invalid_name`, `invalid_agent`, `invalid_message`, `workspace_not_found`, `team_not_found`, `agent_not_found`, `room_not_found`, `agent_has_activity`, `invalid_client_request_id`, `agent_busy`, `client_request_conflict`, `not_set_up`, `invalid_provider`, `invalid_api_key`, `provider_not_configured`, `keychain_unavailable`, `database`); the UI translates it. A run's failures are not command errors: they travel as `error_kind` in the run. Database details do not travel.
- **Database:** the pool is opened in `setup` (`app_data_dir()/meelting.db`) and registered as Tauri state; commands receive it with `State<SqlitePool>`. Repositories accept any `SqliteExecutor` so they can be used inside transactions. Writes that read and then write (create room, send message) open the transaction with `pool.begin_with("BEGIN IMMEDIATE")` on a new pool connection (never nested): they take the write lock at the start and wait up to sqlx's `busy_timeout` (5 s); if it runs out, the error arrives as `database`. Multi-table reads use a normal transaction to read a consistent snapshot.
- **Services without Tauri:** they receive `impl Fn` for output and completion, so they are tested without a window or `AppHandle`.
- **Events to the UI:** emitted by the commands layer. Current names: `terminal-output` (text), `terminal-exit` (exit code), `message:created` (full message with its `room_id`, after the commit), `agent_run:status` (the full run, after the commit of each transition) and `agent_run:token` (`run_id`, `room_id`, `attempt`, `seq`, `text`). The execution engine receives a *callback* (`Emit`) that `commands/agent_runs.rs` translates into Tauri events, so services still know nothing about Tauri.
- **Keys:** the run task reads them from the keychain (`<workspace_id>/<provider>`) right before each attempt, and the provider key services read them only to verify or show their last 4 characters; they never travel to the UI, events or errors. Provider errors are reduced to an `error_kind`; their text never leaves the adapter.
- **Providers without names in the code:** the adapter is chosen with a `match` on the catalog's `protocol` (`system/llm_call.rs`) and receives `base_url`, `model` and the output cap; no model is named in production code.
- **Tests next to the code** (`#[cfg(test)] mod tests`), with real temporary git repositories (`tempfile`) and no external APIs.
- File names in `snake_case`; a service is named after the action (`worktree_run`), not the resource.

## Frontend (`src`)

The UI calls the core **only** with `invoke` and `listen` from `@tauri-apps/api`, and only from hooks.

```
src/
├── main.tsx               Mounts <App /> on #root
├── styles.css             Tailwind v4, Grafito tokens (@theme) with light-dark() and `html[data-theme]` to force light or dark (follows the system by default), Inter and Rubik fonts, and the org chart layout classes (org-*)
├── App.tsx                Chooses between the new workspace page (first launch or "create another") and the shell based on the session
├── assets/fonts/          Inter and Rubik (woff2)
├── i18n/                  es.json, en.json, translate.ts (text, named values, command and run errors, date and agent count) and I18nProvider
├── hooks/                 Core access and associated state
│   ├── useSession.ts      ✅ invoke load_session, complete_first_launch, create_workspace, switch_workspace, change_locale, change_theme, change_ui_scale, change_loading_enabled
│   ├── useTeams.ts        ✅ invoke list_teams, create_team, create_agent, get_agent, update_agent, delete_agent, save_canvas_positions (with a 500 ms debounce), reset_canvas_positions
│   ├── useRooms.ts        ✅ invoke list_rooms, create_room, finish_room
│   ├── useRoom.ts         ✅ invoke get_room, send_intervention (one client_request_id per send, reused in automatic retries and after an IPC timeout; no retries if the recipient is "Nobody") cancel_agent_run, add_room_agents and remove_room_agent; listen message:created, agent_run:status and agent_run:token before loading the room, with events during the load held back
│   ├── useAgentAvailability.ts ✅ invoke check_agent_availability when opening the room, when a run finishes and on request ("Check availability"); never while typing
│   ├── useProviderKeys.ts ✅ invoke list_provider_keys, save_provider_key, check_provider_key, delete_provider_key; keeps the result of each check in memory and never stores the key
│   ├── useRecipientPreview.ts ✅ invoke resolve_recipient 150 ms after the last draft change (local IPC, no network); discards stale responses
│   ├── agentRunState.ts   ✅ pure run state: logical state order (queued n = 2n, running n = 2n − 1, final last), tokens by (attempt, seq), snapshots and live text
│   ├── useAgentTemplates.ts ✅ invoke list_agent_templates (reloads when the language changes)
│   └── useAgentRoles.ts   ✅ invoke list_agent_roles
├── test/setup.ts          Vitest configuration (jest-dom)
└── components/            Atomic design (overview.md §8)
    ├── atoms/             Avatar, Button, CardSelect, Field, Icons, Logo
    ├── molecules/         AgentPicker, AgentRunStatus, AgentTemplateDetail, Alert, ChatMessage, EnginePicker, InviteAgentsForm, IslandMenu, LocalProviderRow, LocaleSwitcher, MentionSuggestions, MessageComposer, Modal, NavItem, OrgAgentCard, OrgCard, OrgDrawer, OrgNode, ParticipantTile, ProviderKeyRow, RoomCard, RoomForm, SegmentedControl, SettingsRow, TransitionOverlay, TeamForm, TeamPicker, ThemeSwitcher, UiScaleSwitcher, WorkspaceSwitcher, ZoomControls; agentLabel.ts omits the role when it repeats the name; mentionDraft.ts detects the mention in progress, filters agents by name or role and inserts the chosen one
    ├── organisms/         AgentCatalogPage, AgentEditForm, AgentForm, NewWorkspacePage, OrgChart, ProvidersPage, Rail, RoomChat, RoomView, RoomsPage, SettingsPage, TeamsPage, Topbar; authorName.ts resolves a message's author among present agents and those who left
    └── templates/         Workspace (rail + topbar + content)
```

| Folder | Responsibility |
|---|---|
| `components/atoms`, `molecules`, `organisms`, `templates` | Presentation. No `invoke`: they receive data and callbacks through props or use a hook. Each level is created when a component needs it |
| `hooks/` | The only place that calls `invoke` or `listen`. One hook per use case, named `use<Verb><Object>` |
| `App.tsx` | Composes organisms or templates; no logic |

Vitest tests live next to the code (`App.test.tsx`, `translate.test.ts`, `RoomsPage.test.tsx`, `RoomView.test.tsx`, `ProvidersPage.test.tsx`, `agentRunState.test.ts`). The `run_in_worktree` spike keeps its command and service in the core with no screen; its interface arrives with Phase 4. The `stream_completion` spike was removed when the `AgentRun` adapters arrived.

## Where to put new things

| I want to add... | It goes in... |
|---|---|
| A command the UI invokes | `commands/<resource>.rs` + registration in `lib.rs` |
| A business action | `services/<action>.rs` |
| A child process, PTY or OS API | `system/<integration>.rs` |
| An SQL query | `repositories/<table>.rs` (requires an approved migration proposal) |
| Pure logic without I/O | `domain/<concept>.rs` |
| A screen or widget | `src/components/<level>/<Name>.tsx` |
| A core call from the UI | `src/hooks/use<Verb><Object>.ts` |
