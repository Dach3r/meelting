# Meelting Desktop

AI workspace / virtual office: the user builds a virtual office with specialized AI agents that collaborate and run real code tasks in isolated `git worktree`s, supervised by the user. 100% local-first desktop app (no server, no account) for macOS, Windows and Linux. Tauri 2 (Rust core, edition 2024) + React 19 + TypeScript + Vite, package manager `pnpm`. It replaces the Rails SaaS (repo `meelting`, frozen as a design reference: domain model, states, errors, copy, templates and specs as test cases).

## Current scope

Phase 0 only: Tauri 2 scaffold, a placeholder "Hello, world" screen (`src/App.tsx`) and the `keychain` Rust module with the commands `save_secret`, `secret_last_four` and `delete_secret` (crate `keyring` 3). Everything else described in `docs/` (SQLite, workspaces, teams, agents, rooms, agent execution, LLM providers, local models, tools, terminal, Docker, approvals, integrations, i18n, Tailwind) is **planned, not built**; do not add it unless asked. Check the real code before assuming a feature exists.

## Ground rules

- Stay in the requested scope; ask before significant product or architecture changes.
- Follow existing conventions and match surrounding style.
- No over-engineering: with two or three cases use a direct conditional, not a generic abstraction. No speculative fields, layers or "just in case" functions.
- **Keep the code clean: no dead code.** Every function, command, constant, variable, component, hook or module you write must be used by the application (a test alone does not count as a use of production code). If something stops being used, delete it in the same change. Do not add code "for later".
- **Never create or edit SQL migrations, or change tables/columns, without presenting the proposal (columns, types, names) and getting explicit approval first.**
- Read `docs/` before changing behavior: `docs/product/` (vision, roadmap, users, virtual office, integrations, metrics; `feature-template.md` defines how to propose features) and `docs/architecture/overview.md` (stack, layers, data model, agent engine).
- Product decisions not yet defined: present alternatives, risks and a recommendation; do not assume.

## Architecture

- Layers in the Rust core: Tauri commands (thin: validate arguments, call a service, return the result; no business rules or SQL) → services (business actions, verbs) → repositories (SQLite access) → pure domain (models, prompt building, provider normalization, path jail) kept free of OS I/O so it can be tested without it. Catalogs are read-only collections embedded in the app (`agent_templates/*.yml`, `llm.yml`); a catalog never orchestrates, an action that uses one is a service. See `docs/architecture/overview.md`.
- Secrets live in the OS keychain through `src-tauri/src/keychain.rs`; the UI never reads a full secret back, only its last four characters.
- The core talks to the UI through Tauri commands and events (`emit`). Declare Tauri capabilities minimal: the webview gets no direct `fs` or `shell` access, only the app's own commands.
- Git and Docker are invoked as child processes through `tokio::process`, not with libraries.
- Frontend: React function components and hooks, TypeScript `strict`. Components follow the atomic design hierarchy of the Rails project. Call the core only through `invoke` from `@tauri-apps/api/core`.
- Validations are mirrored in the database when possible (unique indexes, foreign keys, `CHECK` constraints). Cross-table workspace integrity is validated in services.

## Conventions

- **Internal names are always English** (commands, folders, components, tables, columns; `snake_case` in Rust and SQL). Display text is translated via i18n.
- **i18n** (when built): `es` and `en`; every user-facing string goes in the locale files, no hard-coded UI text. Spanish copy uses "correo electrónico", not "email".
- **Migrations** (when built): `sqlx migrate`, one `create_table` per file, named `create_<table>`; other changes get their own `add_<column>_to_<table>` file. Never edit a committed migration.
- **Code comments**: do not write them. Keep code self-explanatory.
- **Secrets**: never commit `.env` or keys (use `.env.example` for new variables).
- **Docs**: a behavior change updates its document in `docs/` in the same commit. Docs are in Spanish; this file is in English.

## Tests

- Rust: `cargo test` (run from `src-tauri`). Frontend: Vitest. E2E: WebDriver with `tauri-driver`. Test runners are mostly pending; add them with the first feature that needs them.
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
