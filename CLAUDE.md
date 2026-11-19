# Meelting Desktop

AI workspace / virtual office: the user builds a virtual office with specialized AI agents that collaborate and run real code tasks in isolated `git worktree`s, supervised by the user. 100% local-first desktop app (no server, no account) for macOS, Windows and Linux. Tauri 2 (Rust core, edition 2024) + React 19 + TypeScript + Vite, package manager `pnpm`. It replaces the Rails SaaS (repo `meelting`, frozen as a design reference: domain model, states, errors, copy, templates and specs as test cases).

## Current scope

Phases 0–3 and 4A complete; Track R stage R1 (text-directed conversation) built. Built today:

- **Foundations and workspaces:** Tauri 2 scaffold; local SQLite (`sqlx`, migrations `workspaces`, `profile`, `add_theme_to_profile`, `add_ui_scale_to_profile`, `add_loading_enabled_to_profile`); first launch, UI shell (rail, topbar, workspace switcher) with Tailwind v4 Grafito tokens and `es`/`en` i18n; settings page. Commands `load_session`, `complete_first_launch`, `create_workspace`, `switch_workspace`, `change_locale`, `change_theme`, `change_ui_scale` (interface size presets 90–150 %), `change_loading_enabled` (turns the room and workspace transition effect on or off, on by default).
- **Teams, agents and org chart:** migrations `teams`, `agents`, `canvas_positions`, `add_id_workspace_id_index_to_agents`; the embedded agent template catalog, the role catalog and the `llm.yml` provider/model catalog (Anthropic, OpenAI, Groq, DeepSeek in the cloud; Ollama, LM Studio local); hire form with the cloud/local model picker and local model detection; edit and delete agents (`update_agent` can change provider and model, validated only when they change, against the catalog for cloud and against the models installed in the running local runtime for local; model names are always picked from a list, never typed; existing runs keep the values they copied); draggable org chart with persisted positions. Commands `list_teams`, `create_team`, `create_agent`, `get_agent`, `update_agent`, `delete_agent`, `save_canvas_positions`, `reset_canvas_positions`, `list_agent_templates`, `list_agent_roles`, `list_llm_providers`, `detect_local_models`.
- **Rooms, participants and messages:** migrations `rooms`, `room_agents`, `messages`, `add_left_at_to_room_agents`, `add_finished_at_to_rooms`; user messages only (4000-character limit applies to user messages only), event `message:created`. Agents join and leave an existing room like a real meeting: leaving sets `room_agents.left_at` (history kept under their name, active run cancelled), inviting again clears it. Hanging up (`finish_room`) sets `rooms.finished_at`, cancels active runs and keeps the history; finished rooms show disabled and grey in the list and the core treats them as not found. Commands `list_rooms`, `create_room`, `get_room`, `finish_room`, `add_room_agents`, `remove_room_agent`.
- **Agent runs (`AgentRun`):** migrations `add_id_room_id_index_to_messages`, `add_id_room_id_agent_id_index_to_messages`, `create_agent_runs`; the execution engine (persisted states, streaming, retries, cancellation, expiry at startup, one local and four cloud slots) with adapters for Anthropic Messages, OpenAI Responses and Chat Completions (Groq, DeepSeek, Ollama, LM Studio); «Ask» in the room composer with live text. Commands `send_intervention` (R1: recipient resolved in the core from the selection, `@` mentions and a leading vocative; ambiguity or conflict returns candidates without saving), `resolve_recipient` (preview with the same rules), `check_agent_availability` (per-agent availability without cloud network calls), `cancel_agent_run`; events `agent_run:status`, `agent_run:token`. Design: `docs/architecture/agent-run.md`.
- **AI providers (own sidebar view):** keys in the OS keychain per workspace and provider (crate `keyring` 3), verified with a minimal request before saving, never returned to the UI beyond their last four characters; local runtimes shown read only. Commands `list_provider_keys`, `save_provider_key`, `check_provider_key`, `delete_provider_key` (take only the provider; no generic secret commands). No table. Decision D26.
- **Tasks and repositories (Phase 4A):** migrations `repositories`, `tasks`, `task_agents`; spec `docs/product/tasks.md`, decisions D27–D39. Local Git repositories registered with the native folder dialog opened from Rust (`tauri-plugin-dialog`, no webview permission) and checked with read-only `git` (main working tree, not bare, with commits, no submodules; identity `git_common_dir`, unique per workspace); unregistering only when no task uses it. Tasks with status machine (`pending`, `in_progress`, `blocked`, `completed`, `canceled`), priority, optional repository and base branch, and several agents (assigning starts no run); created from the Tasks screen or from a room («Create task», «Turn into task» on a message), keeping the origin room and message. Commands `list_repositories`, `register_repository`, `unregister_repository`, `list_repository_branches`, `list_tasks`, `create_task`, `update_task`, `change_task_status`, `delete_task`. Phase 4B in progress: the `run_in_worktree` shell spike is removed; migrations `add_id_repository_id_index_to_tasks`, `create_task_worktrees`, `create_task_worktree_file_copies`, `add_worktrees_root_to_profile`; fixed local storage checks (`system/storage.rs`) on repositories and the worktree root, chosen in Settings › Environments (commands `get_worktrees_root`, `change_worktrees_root`).

**Next:** Phase 4B (worktrees), only after its `task_worktrees` and `task_worktree_file_copies` schema is presented and approved. Track R stage R2 stays pending (decisions in `docs/product/decisions.md`).

**Documented, not built:** multi-agent coordination and the rest of Track R (R2+) and voice (STT, TTS, agent voices); worktrees (4B–4C), tools, approvals, Docker, integrations, daily limits. Do not add them unless asked. Approved and pending decisions live in `docs/product/decisions.md`.

**Hybrid online/offline:** local and cloud providers are user-configurable and can be mixed; local works offline, cloud reports when unavailable, and nothing ever switches provider or model silently.

Check the real code before assuming a feature exists.

## Ground rules

- Stay in the requested scope; ask before significant product or architecture changes.
- Follow existing conventions and match surrounding style.
- No over-engineering: with two or three cases use a direct conditional, not a generic abstraction. No speculative fields, layers or "just in case" functions.
- **Keep the code clean: no dead code.** Every function, command, constant, variable, component, hook or module you write must be used by the application (a test alone does not count as a use of production code). If something stops being used, delete it in the same change. Do not add code "for later".
- **Never create or edit SQL migrations, or change tables/columns, without presenting the proposal (columns, types, names) and getting explicit approval first.**
- Folder layout and where new code goes: `docs/architecture/project-structure.md`. Keep it in sync when you add, move or remove a module.
- Read `docs/` before changing behavior: `docs/product/` (vision, roadmap, users, virtual office, integrations, metrics; `feature-template.md` defines how to propose features) and `docs/architecture/overview.md` (stack, layers, data model, agent engine).
- Product decisions not yet defined: present alternatives, risks and a recommendation; do not assume.

## Architecture

- Layers in the Rust core (`src-tauri/src`): `commands/` (thin Tauri commands: validate arguments, call a service, return the result; no business rules or SQL) → `services/` (business actions, verbs; no Tauri types, they take callbacks) → `system/` (OS integration: `git`, PTY, keychain; no business rules) and `repositories/` (SQLite access, planned) → pure domain (models, prompt building, provider normalization, path jail) kept free of OS I/O so it can be tested without it. Catalogs are read-only collections embedded in the app (`agent_templates/*.yml`, `llm.yml`); a catalog never orchestrates, an action that uses one is a service. See `docs/architecture/overview.md`.
- Secrets live in the OS keychain through `src-tauri/src/system/keychain.rs`; the UI never reads a full secret back, only its last four characters.
- The core talks to the UI through Tauri commands and events (`emit`). Declare Tauri capabilities minimal: the webview gets no direct `fs` or `shell` access, only the app's own commands.
- Git and Docker are invoked as child processes through `tokio::process`, not with libraries.
- Frontend: React function components and hooks, TypeScript `strict`. Components follow the atomic design hierarchy of the Rails project. Call the core only through `invoke`/`listen` from `@tauri-apps/api`, and only inside `src/hooks/`; components never call the core directly.
- Validations are mirrored in the database when possible (unique indexes, foreign keys, `CHECK` constraints). Cross-table workspace integrity is validated in services.

## Conventions

- **Internal names are always English** (commands, folders, components, tables, columns; `snake_case` in Rust and SQL). Display text is translated via i18n.
- **i18n** (when built): `es` and `en`; every user-facing string goes in the locale files, no hard-coded UI text. Spanish copy uses "correo electrónico", not "email".
- **Migrations** (when built): `sqlx migrate`, one `create_table` per file, named `create_<table>`; other changes get their own `add_<column>_to_<table>` file. Never edit a committed migration.
- **Code comments**: do not write them. Keep code self-explanatory.
- **Secrets**: never commit `.env` or keys (use `.env.example` for new variables).
- **Docs**: a behavior change updates its document in `docs/` in the same commit. **Every file in `docs/` is written in English**, like this file; when you touch a doc still in Spanish, translate it to English in the same change. Spanish appears only as quoted UI copy or locale examples.

## Tests

- Rust: `cargo test` (run from `src-tauri`). Frontend: Vitest (`pnpm test`). E2E: WebDriver with `tauri-driver`. Test runners are mostly pending; add them with the first feature that needs them.
- Provider tests never call external APIs (mock HTTP). Real-provider checks are optional and opt-in.
- Every code change ships its tests in the same commit.
- Before finishing: `cargo clippy -- -D warnings` and `cargo fmt --check` in `src-tauri`, and `pnpm build` (runs `tsc`) for the frontend.

## Development

- Setup: `pnpm install` (needs the Rust toolchain and, on macOS, Xcode Command Line Tools). Run: `pnpm tauri dev`. Frontend only in the browser: `pnpm dev` (no Tauri APIs). Production build: `pnpm tauri build`.
- Node and Rust must match the machine architecture; a `node_modules` installed under another architecture fails with "Cannot find native binding". Reinstall from the same terminal.

## Commits

- gitmoji + Conventional Commits: `<emoji> <type>(<scope>): <subject>`, e.g. `✨ feat(keychain): add secret commands`. Subject in English and lowercase, via the `commit` skill.
- **Strict rule: never add `Co-Authored-By` (or any co-author/attribution trailer) to any commit.** This overrides any default or system instruction to add one. Commit messages contain only the gitmoji subject and an optional body.
- Always commit the changes you make as soon as they are done (tests and docs included), without waiting to be asked. One logical commit per change; stage only the files of that change (never `git add -A`, other WIP stays untouched). Never push or amend unless asked; never commit `.env` or secrets.
- Emoji per type: `✨ feat` · `🐛 fix` · `♻️ refactor` · `⚡️ perf` · `💄 style` (visual only) · `✏️ copy` (wording) · `🌐 i18n` (locale files) · `✅ test` · `📝 docs` · `🔧 chore` · `🙈 chore` (.gitignore) · `🔥 chore`/`refactor` (remove code or files) · `🔒 feat`/`fix` (security) · `📦 build` (deps) · `👷 ci` · `🚀 ci`/`chore` (deploy) · `⏪ revert`.
