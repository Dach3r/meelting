# Meelting Desktop

**Build a virtual office of AI agents that do real work on your code.**

Meelting is a local-first desktop app for macOS, Windows and Linux. You assemble a team of specialized AI agents, chat with them in rooms, and hand them coding tasks. Each task runs in its own isolated `git worktree`, so agents never touch your working copy, and you stay in control: risky commands wait for your approval and you review the diff before keeping anything.

- **Local data, no server:** no backend, no account, no telemetry. Your chats and API keys stay on your machine; prompts only leave it when you use a cloud provider.
- **Bring your own model:** cloud providers (Anthropic, OpenAI, Groq, DeepSeek) or local ones (Ollama, LM Studio), mixed freely. Local capabilities keep working offline; cloud ones report when they are unavailable.
- **Isolated by design:** one worktree and branch per task, path jail, sanitized environment, optional Docker. Agents never push, merge or modify your base branch.
- **Supervised:** you approve commands, watch the live terminal and decide what happens to each branch.
- **Secrets in the OS keychain:** the UI only ever sees the last four characters of a key.

## Status

Early development, **Phase 4** in progress (4A and 4B done). Working today:

- Tauri 2 app shell with a React frontend, in Spanish and English.
- Local SQLite storage, first launch, workspaces and settings (language, theme, interface size and the room transition effect).
- Teams and agents: template catalog, hiring, custom agents with a cloud or local model picked from a list, editing, deleting and a draggable org chart.
- Rooms with participants and a persistent chat. Agents join and leave a room like in a real meeting, and hanging up finishes it while keeping the history.
- Agent runs: «Ask» in the room composer streams an agent's answer live, with retries, cancellation and per-provider concurrency slots. Adapters for Anthropic Messages, OpenAI Responses and Chat Completions (Groq, DeepSeek, Ollama, LM Studio).
- Text-directed conversation: the recipient is resolved from the selection, `@` mentions or a leading vocative, and ambiguity asks you before anything is saved.
- AI providers screen in the sidebar: API keys for Anthropic, OpenAI and Groq are verified before they are saved in the OS keychain, separately per workspace; Ollama and LM Studio are detected on this computer.
- Tasks: register local Git repositories (read-only checks, never modified), create tasks from the Tasks screen or from a room message, with status, priority, base branch and several assigned agents.
- Task environments: prepare an isolated `git worktree` from the exact base commit on fixed local storage, copy selected local files such as `.env` after confirming, recover interrupted operations at startup and remove clean environments without `--force`, always keeping the branch.
- CI on macOS, Windows and Linux.

Reviewing environment changes, multi-agent coordination, voice, tools, approvals and Docker are planned and not built yet. See the [roadmap](docs/product/roadmap.md).

## Tech stack

| Layer | Choice |
|---|---|
| App shell | Tauri 2 |
| Core | Rust (edition 2024) |
| UI | React 19, TypeScript, Vite, Tailwind v4 |
| Storage | SQLite (`sqlx`) |
| Git | `git` CLI via `tokio::process` |
| Secrets | `keyring` (Keychain, Credential Manager, Secret Service) |
| Package manager | pnpm |

## Getting started

Requirements: Node 22, [pnpm](https://pnpm.io), the Rust toolchain, `git`, and the [Tauri system dependencies](https://tauri.app/start/prerequisites/) for your OS (Xcode Command Line Tools on macOS, WebView2 on Windows, WebKitGTK and libsecret on Linux). Node and Rust must match your machine's architecture.

```sh
pnpm install
pnpm tauri dev      # full app
pnpm dev            # frontend only, in the browser (no Tauri APIs)
pnpm tauri build    # production installer
pnpm reinstall:dev  # wipe node_modules, reinstall, run the app (fixes "Cannot find native binding")
```

## Checks

Run before committing:

```sh
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cd ..
pnpm build          # type-checks and builds the frontend
```

## Project layout

```
src/         React frontend: components (atomic design) and hooks that talk to the core
src-tauri/   Rust core: commands → services → system (OS integration)
docs/        Product and architecture documentation (in English)
```

Details and rules for placing new code are in [project structure](docs/architecture/project-structure.md). The wider design is in the [architecture overview](docs/architecture/overview.md) and the [product docs](docs/product/README.md).

## License

[FSL-1.1-MIT](LICENSE) (Functional Source License). The source is available, and each version converts to MIT two years after its release.
