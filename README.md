# Meelting Desktop

**Build a virtual office of AI agents that do real work on your code.**

Meelting is a local-first desktop app for macOS, Windows and Linux. You assemble a team of specialized AI agents, chat with them in rooms, and hand them coding tasks. Each task runs in its own isolated `git worktree`, so agents never touch your working copy, and you stay in control: risky commands wait for your approval and you review the diff before keeping anything.

- **Local-first:** no server, no account, no telemetry. Your code, chats and API keys stay on your machine.
- **Bring your own model:** cloud providers (OpenAI, Anthropic, DeepSeek, MiniMax) or local ones (Ollama, LM Studio). With only a local model the app works fully offline.
- **Isolated by design:** one worktree and branch per task, path jail, sanitized environment, optional Docker. Agents never push, merge or modify your base branch.
- **Supervised:** you approve commands, watch the live terminal and decide what happens to each branch.
- **Secrets in the OS keychain:** the UI only ever sees the last four characters of a key.

## Status

Early development, **Phase 1** (foundations). Working today:

- Tauri 2 app shell with a React frontend.
- Local SQLite storage, first launch (language and workspace), app shell and workspace switching, in Spanish and English.
- Keychain commands: save, read the last four characters of, and delete a secret.
- Run a command in a fresh `git worktree` with live PTY output (a spike, not the final task flow).
- CI on macOS, Windows and Linux.

Everything else (teams, agents, rooms, agent execution, LLM providers, tools, approvals, Docker) is planned and not built yet. See the [roadmap](docs/product/roadmap.md).

## Tech stack

| Layer | Choice |
|---|---|
| App shell | Tauri 2 |
| Core | Rust (edition 2024) |
| UI | React 19, TypeScript, Vite, Tailwind v4 |
| Storage | SQLite (`sqlx`) |
| Terminal | `portable-pty` (ConPTY on Windows) |
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
docs/        Product and architecture documentation (in Spanish)
```

Details and rules for placing new code are in [project structure](docs/architecture/project-structure.md). The wider design is in the [architecture overview](docs/architecture/overview.md) and the [product docs](docs/product/README.md).

## License

[FSL-1.1-MIT](LICENSE) (Functional Source License). The source is available, and each version converts to MIT two years after its release.
