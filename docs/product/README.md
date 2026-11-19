# Meelting Desktop — Product

**Category:** AI Workspace / Virtual AI Office / Multi-Agent Collaboration
**Platform:** desktop application (Tauri 2) for macOS, Windows and Linux, with no backend of its own and no account, with hybrid online/offline operation
**Stage:** Phase 0 complete (Tauri 2 scaffold with keychain, `git worktree` + PTY and SSE streaming spikes); Phase 1 complete (SQLite, first launch, shell and workspaces); Phase 2 complete (teams and agents); Phase 3 in progress (rooms and messages built, without agent runs)

> **Your artificial intelligence team working with you, in a virtual office.**

Meelting Desktop is an application that runs on the user's machine, with no server of its own and no account: its data lives locally. The user creates a virtual office with AI agents that work autonomously and run real code tasks in isolated `git worktree`s, with cloud or local models (Ollama, LM Studio) that can be combined. Local works offline; whatever depends on cloud services reports when it is unavailable. The user is the team lead: supervises, assigns goals, converses and approves actions.

This product replaces the original Rails SaaS (repository `meelting`), which stays frozen as a design reference. See [../architecture/overview.md](../architecture/overview.md).

## Index

| Document | Content |
|---|---|
| [vision.md](vision.md) | Vision, problem, value proposition, principles |
| [virtual-office.md](virtual-office.md) | Virtual office concept: agents, rooms, screen sharing, chat, approvals |
| [agent-catalog.md](agent-catalog.md) | Template catalog, teams and agent management |
| [rooms.md](rooms.md) | Work rooms: goal, participants and persistent conversation |
| [agent-execution.md](agent-execution.md) | Agent runs: providers (cloud and local), streaming, states, limits and errors |
| [voice-meetings.md](voice-meetings.md) | Conversational meetings with agents: modes, turn coordination, voice (proposal) |
| [settings.md](settings.md) | Settings: sections, groups, defaults and behavior |
| [decisions.md](decisions.md) | Log of approved, pending and discarded decisions |
| [users.md](users.md) | Target users and main experience (journey) |
| [roadmap.md](roadmap.md) | Phases, scope and what is not built |
| [integrations.md](integrations.md) | Planned integrations (GitHub, Slack, ClickUp) |
| [metrics-and-business.md](metrics-and-business.md) | Success indicators and business model |
| [feature-template.md](feature-template.md) | Standard format for documenting features and decision rules |

Technical architecture: [../architecture/overview.md](../architecture/overview.md).

## Information status

Each document distinguishes between:

- **Implemented:** exists today in this repository's code.
- **Planned:** defined in the migration plan and not built yet. Feature documents (catalog, rooms, runs) describe the **target** behavior, ported from the Rails product, where it was already validated.
- **Vision:** product direction, with no delivery commitment.
- **Hypothesis:** to be validated.
- **Proposal:** documented design, pending approval for implementation (e.g. [voice-meetings.md](voice-meetings.md)).

**Implemented today:** Tauri 2 scaffold (React + TypeScript + Vite) (the Phase 0 spikes were replaced by the real features or removed); SQLite, first launch, shell, workspaces, language and theme settings; teams, agents, org chart, template catalog and provider and model picker with local model detection; rooms, participants and user messages. Agent runs, conversational meetings and voice are planned or proposed.

## Core principle

Meelting does not aim to simulate an AI working. It aims to let AIs actually work, while the user observes, collaborates and stays in control. The virtual office, the avatars and the rooms are the interface that makes that work understandable. The real product is the ability to **delegate, coordinate, supervise and complete real tasks** through AI agents.
