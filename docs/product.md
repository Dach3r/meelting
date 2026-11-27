# Meelting — Product Vision

> Source of truth for what Meelting is, why it exists, what has been decided, and what to build first.
> Last updated: 2026-11-27 · Owner: David

## How to use this document (for Claude Code)

- **This document defines the product.** Read it before planning or writing code. When a request conflicts with it, point out the conflict and ask before proceeding.
- **Decision status matters.** In the Decisions table:
  - **Decided**: build on it without asking.
  - **Proposed**: use it as the default, but confirm with David before work that is hard to undo (database schema, infrastructure, public interfaces).
  - **Open**: do not assume an answer. Ask.
- **Technical conventions live in `CLAUDE.md`.** Stack, testing, linting, i18n, migrations, architecture layers and commit format are defined there and take precedence. This document does not duplicate them. When Meelting hosts itself, `CLAUDE.md` stays hand-written (see "Project Memory").
- **Stay inside the current milestone.** Anything listed under "Out of scope" needs explicit approval, even if it looks small.
- **Dogfood Project Memory.** When a decision is made during the work, record it as a decision record in `.project/decisions/` once that structure exists (see "Project Memory"). Until then, update the Decisions table here.
- **Ask instead of guessing.** If something here is ambiguous or missing, ask a specific question with options rather than inventing context.

## Vision

Meelting is an open source, self-hostable code collaboration platform where humans and AI agents understand, build and maintain software together. It provides shared project memory, controlled autonomy, and a traceable record of what every agent does.

Meelting is an experiment, not a commercial product for now. The goal is to build it, use it daily on real projects, and see whether it gains traction. Monetization is out of scope. The AGPL-3.0 license keeps the option of a hosted instance open for the future.

The promise: a developer should not have to explain to an AI how their project works every time a task starts. The repository holds the project's knowledge, not just its code.

## The problem

Agents can already research, implement and test, but they work without the project's accumulated knowledge and outside the flow where the code lives.

- Code, tasks, documentation and decisions live in separate systems.
- Each agent session rebuilds context from scratch, and what it learns is lost when it ends.
- An agent can implement something correct that contradicts an architectural decision it never saw.
- Agent output is integrated into the pull request and review flow by hand.

The problem is not generating code. It is getting AI to understand the project, take part in its process, and leave that knowledge available for the next task.

## Landscape and differentiation

Multi-agent orchestration, agent identity and the issue-to-PR flow already exist on GitHub and GitLab. Meelting implements them because they are necessary, not because they differentiate it.

| Platform | What it already offers | Conditions |
| --- | --- | --- |
| [GitHub Agent HQ](https://github.blog/news-insights/company-news/welcome-home-agents/) (October 2025) | Mission control to assign, steer and track multiple agents; agents from Anthropic, OpenAI, Google, Cognition and xAI; per-agent identity, access control and branch controls | [Included with paid Copilot subscriptions](https://www.directionsonmicrosoft.com/reports/github-agent-hq-brings-agent-governance-third-party-support/); repositories on GitHub |
| [GitLab Duo Agent Platform](https://docs.gitlab.com/user/duo_agent_platform/) | Foundational, custom and external agents; flows; a [Knowledge Graph](https://docs.gitlab.com/user/project/repository/knowledge_graph/) (beta) that models files, classes, functions and their relationships | Repositories on GitLab |

What Meelting bets on, which neither offers today:

1. **Memory of the "why", versioned in the repository.** Decisions, conventions and constraints live in files that travel with the code, with traceable sources, readable by any agent, even outside Meelting.
2. **Open source and self-hosted, with any model.** No vendor subscription; users bring their own API keys (BYOK).
3. **A platform designed around agent work.** This is the weakest bet, because GitHub and GitLab are moving in the same direction. It only counts if it shows in daily use.

## Principles

1. **AI-native, not AI-added.** Agents have contextual access to the project and take part in its processes. They are not a chat window next to a repository.
2. **A repository is knowledge, not just code.** What the system does, how it is organized, why decisions were made, and which conventions it follows.
3. **Humans and agents as collaborators.** Every agent has an identity, permissions and a visible activity history.
4. **Autonomy with control.** Users decide what each agent may do and which actions require approval. No black boxes.
5. **Compatible with Git and existing tools.** Standard Git, and memory that Claude Code or Codex can read without any integration.
6. **Open.** AGPL-3.0 code, self-hosted first, any model provider.

## Glossary

| Term | Meaning |
| --- | --- |
| Repository | A Git repository hosted by Meelting, stored as a bare repo on disk |
| Agent | An AI participant with an identity and permissions. Either an external CLI agent (Claude Code, Codex) or, later, Meelting's own runtime |
| Agent run | One execution of an agent on one task (for example, an issue), inside its own sandbox |
| Run event | One step of a run (file read, edit, command, message), stored and streamed live to the UI |
| Sandbox | A Docker container created for a single run and destroyed afterwards |
| Approval gate | A point where a run stops until a human approves (in the first milestone: before opening the PR) |
| Project Memory | The project's knowledge, stored as versioned Markdown in `.project/` and indexed by Meelting |
| Decision record | One file in `.project/decisions/` describing a decision, its context, consequences and sources |
| BYOK | Bring your own key: model API keys belong to the user, not to Meelting |
| Inbox | The authenticated home: the list of things that need a human decision (see "Authenticated home and navigation") |

Suggested core entities for the first milestone. This is a starting point, not a schema: propose the actual schema to David before creating migrations.

| Entity | Purpose |
| --- | --- |
| User | Account and authentication |
| AccessToken | Personal token for Git over HTTPS and API access |
| Repository | Metadata for a hosted Git repository |
| Mirror | Push mirror configuration and its credential (GitHub) |
| Issue | A task with title, description, state and comments |
| PullRequest | A request to merge one branch into another in the same repository |
| Comment | Discussion on an issue or pull request |
| Agent | An agent definition: kind, name, permissions |
| AgentRun | One run of an agent on an issue: state, branch, sandbox, result, approval |
| RunEvent | Ordered steps of a run |
| ProviderCredential | A user's encrypted model API key (BYOK) |
| Memory index tables | Derived index of `.project/` content; shape defined with the indexer |

## Decisions

| Decision | Choice | Rationale | Status |
| --- | --- | --- | --- |
| Nature | Open source experiment, no commercial goal for now | Find out whether it works and gains traction before thinking about a business | Decided |
| Git hosting | Self-hosted: bare repos on disk, Smart HTTP via `git http-backend` and the git CLI | Full control of the agent workflow; the cost is building a forge's foundation | Decided |
| Stack | Rails 8 + Hotwire monolith; Go only for a concrete need | Most productive stack for a solo developer; GitLab proved a Rails forge works | Decided |
| Database | PostgreSQL | Full-text search for the memory index; pgvector later if semantic search is needed | Decided |
| Technical conventions | Defined in `CLAUDE.md`, which takes precedence over this document | Already agreed; one source avoids drift | Decided |
| Document location | `docs/product.md` | `CLAUDE.md` keeps documentation in `docs/` with kebab-case names | Decided |
| Agents | Own runtime (RubyLLM, BYOK) + Claude Code and Codex in headless mode, all in Docker sandboxes | Own control without giving up the best existing agents | Decided |
| Agent order | Claude Code headless first; own runtime after the first milestone | A high-quality agent from day one without writing an agent loop; shortest path to daily use | Proposed |
| Memory | Versioned files in `.project/` as the source of truth, indexed in the database | Traceable, portable, and readable by external agents without integration | Decided |
| `AGENTS.md` / `CLAUDE.md` ownership | Hand-written files with a generated block between markers; Meelting rewrites only that block. `CLAUDE.md` imports `AGENTS.md` and adds its own rules | The repo keeps hand-written conventions (as Meelting's own does) while memory summaries stay fresh | Decided |
| Deployment | Docker Compose is the official path for self-hosters; Kamal deploys David's own instance | Self-hosting is the foundation of an open source project; Kamal is already in the stack | Decided |
| Public instance timing | After the first milestone | The most expensive and risky piece, and the success criterion does not need it | Proposed |
| First milestone | Meelting hosts itself | Shortest path to the success criterion | Decided |
| Merge from the UI | Merge commit only, built with `git merge-tree` + `git commit-tree` on the bare repo (no worktree), with a per-repository lock against concurrent pushes | Avoids worktree I/O and cleanup; conflicts are reported, not resolved, in v0 | Decided |
| License | AGPL-3.0 | Asymmetric: can move to MIT later, not the other way; protects a future hosted instance | Decided |
| Contributions | DCO (`Signed-off-by`), no CLA | Low friction. Trade-off: relicensing later needs consent from every contributor, so keep external contributions minimal until the license question is final | Decided |
| Success criterion | Daily use on David's own projects | A verifiable signal that does not depend on third parties | Decided |
| Weekly hours | Not fixed on purpose; no time commitment | The roadmap carries no dates or estimates | Decided |
| Evaluation date | Not set | Without a date the experiment is never evaluated (see risks) | Open |
| Sandbox network policy | Not set | Claude Code needs to reach the model provider, so "no network" is not an option; decide before Phase 3 | Open |
| BYOK key inside the sandbox | Not set | Environment variable vs a proxy that injects the key outside the container; see risks | Open |
| App navigation | Persistent sidebar: Inbox, Runs, repositories (each expands in place to Code, Issues, Pull requests, Runs, Memory) and Settings | With one or two repositories in the first milestone, expanding in place keeps everything one click away without an extra tab level | Decided |
| Authenticated home | The home is the Inbox: things that need a decision, plus runs in progress. Nothing else | Notifications are out of scope, so the home is the only place a run waiting at the gate is seen; anything that is not a decision makes it harder to see | Decided |
| Rejected run | The run stays in history as rejected and leaves the Inbox; its issue goes back to pending, ready to reassign. No automatic retry | Keeps the gate simple and inside the milestone; retry with feedback can come later | Decided |
| Home placeholder data | The navigation shell and home are built before Phases 2–3 using placeholder data from one clearly labelled source; no models or migrations for future-phase entities | Lets the layout be built and reviewed early without creating schema ahead of its phase | Proposed |
| Visual style | shadcn/ui-inspired look (zinc neutrals, 1px borders, 8px radius, Geist) built with Hotwire, without React | Quiet and dense for daily use; the look is adopted, not the React components | Proposed |

## Architecture

One Rails monolith covers the UI, Git and orchestration. Code executed by agents runs isolated, in one container per run.

### Components

```
Clients                 Rails 8 monolith                         Isolated execution
-------                 ----------------                         ------------------
Browser  ───────────▶   UI + API (Hotwire, Turbo Streams)
Git client (HTTPS) ──▶  Git Smart HTTP (git http-backend, git CLI)
SSH (later, Go)  ┄┄┄▶   Agent runs (Solid Queue jobs)  ────────▶ Docker sandbox per run
                        Memory index (re-indexes .project/)         - external agents: Claude Code, Codex
                                                                    - tools: shell, tests, file edits
                                │                     │                        │
                                ▼                     ▼                        ▼
                        Bare repos on disk      PostgreSQL              Model providers (BYOK)
                        (code + .project/)      (data + index)          Anthropic, OpenAI, Ollama
```

- **Git.** Smart HTTP through `git http-backend`; trees, diffs and blame through the git CLI. Merges use `git merge-tree` and `git commit-tree` under a per-repository lock. SSH is deferred to a separate Go service, the first component outside Rails.
- **Agents.** Solid Queue jobs orchestrate runs. The own agent loop (later) runs in the job with RubyLLM and executes its tools inside the sandbox. External agents run as CLI processes inside the sandbox.
- **Why Ruby is enough.** There are official Anthropic and MCP SDKs for Ruby, and RubyLLM covers multiple providers. The Claude Agent SDK does not exist for Ruby (Python and TypeScript only), but Meelting does not need it: external agents run as CLIs.
- **Self-hosting.** A Docker image of the app + PostgreSQL + a Docker daemon for sandboxes. Mounting the Docker socket into the app is equivalent to root access on the host, so use a separate runner or a socket proxy with limited permissions.

### Agent run lifecycle (first milestone)

1. A user assigns an issue to an agent.
2. A Solid Queue job creates an AgentRun, starts a sandbox container, clones the repository and creates a new branch.
3. The agent (Claude Code in headless mode, using the user's API key) works on the issue with `.project/` available as context.
4. Every step is stored as a RunEvent and streamed to the UI with Turbo Streams.
5. When the agent finishes, the run waits at the approval gate.
6. On approval, Meelting pushes the branch and opens a pull request with a structured description. On rejection, the run is kept as rejected and its issue goes back to pending.
7. The sandbox is destroyed.

A pull request opened by an agent includes: objective, summary of changes, files affected, decisions made, tests run and their results, risks identified, and proposed changes to `.project/`.

## Project Memory

Memory lives in the repository as versioned Markdown files; Meelting only indexes it. If someone leaves Meelting, the memory leaves with their code.

Proposed structure for v0:

```
.project/
  overview.md        what the system does and for whom
  architecture.md    modules, responsibilities and how they relate
  conventions.md     team rules and code conventions
  known-issues.md    technical debt, risks and known constraints
  decisions/
    0001-<title>.md  context, decision, consequences and sources
AGENTS.md            hand-written + generated block (summary and links to .project/)
CLAUDE.md            imports AGENTS.md, adds its own hand-written rules
```

Rules:

- **Traceability.** Every decision links to its sources: the issue, pull request or commit that produced it. A claim without a source is marked as such.
- **Agents propose, humans approve.** An agent that learns something updates `.project/` in its own pull request, and the change goes through review like any other.
- **Derived index.** A post-receive hook enqueues a job that re-indexes `.project/` in PostgreSQL. The index can be deleted and rebuilt from the repository at any time.
- **Compatibility without integration.** `AGENTS.md` and `CLAUDE.md` let Claude Code, Codex and other agents read the memory even when they run outside Meelting.
- **Generated block.** Meelting rewrites only the section between its markers in `AGENTS.md`; everything outside it is hand-written and never touched.
- **Conventions.** Until Meelting generates it, `.project/conventions.md` points to the hand-written conventions in `CLAUDE.md` rather than duplicating them.

Open question: the exact format of each file (frontmatter, required fields of a decision record) must be defined before writing the indexer.

## Authenticated home and navigation

The home answers one question: what needs me right now? Notifications are out of scope in the first milestone, so the home is the only place where a finished, failed or waiting run is seen. Anything that is not a pending decision lives somewhere else.

### Sidebar

Always visible for an authenticated user, top to bottom:

| Item | Shows | Notes |
| --- | --- | --- |
| Inbox | Badge with the number of Inbox items | This is the home (`/`) |
| Runs | Dot with the number of runs in progress | Global list of runs; the weekly summary lives in its header |
| Repositories | One entry per repository, with a mirror status dot (ok, failing, none) | Clicking a repository expands it in place: Code, Issues, Pull requests, Runs, Memory. Only one repository is expanded at a time |
| New repository | Link | |
| Settings | Warning dot when a configuration problem exists | Access tokens, model keys, mirrors |
| Account | User name, sign out | |

### Home blocks

Exactly two blocks, in this order.

**1. Inbox.** One list, sorted by how long each item has been waiting (oldest first). Each row has a type label, a title, one context line (repository · agent · relative time), at most one flag, and one action.

| Type | When it appears | Flag (only when true) | Action | Leaves the Inbox when |
| --- | --- | --- | --- | --- |
| Approve | A run is waiting at the approval gate | "Tests failed" | Review → run page | Approved or rejected |
| Merge | An agent pull request is open | "Modifies .project/" or "Has conflicts" | Open → pull request | Merged or closed |
| Failed | A run ended with an error (tests, sandbox timeout, agent CLI error) | — | View run → run page | Dismissed, or a new run starts on the same issue |

Details (diff size, cost, tests run, memory used) belong on the run and pull request pages, not on the row.

**2. In progress.** Up to three running runs, each with its issue, elapsed time and last run event, updated live. A "View all" link goes to Runs when there are more.

### States

- **Empty Inbox.** "Nothing needs you" with one action: "Assign an issue", which goes to the repository's open issues.
- **First visit.** Until setup is complete, a checklist replaces both blocks: create an access token, create a repository and push to it, configure the GitHub mirror, add a model key, write `.project/overview.md`, assign a first issue to an agent. Each step is checked automatically when it is true. The checklist never appears next to the Inbox.
- **Configuration problem.** A missing model key, an unreachable Docker daemon or a failing mirror shows a dot on Settings (or on the repository) and, only when it blocks runs or puts the repository at risk, one banner above the Inbox with a link to fix it.

### Not on the home

- Activity feeds (commits, comments, pushes).
- "Assigned to me" filters: there is a single user in the first milestone.
- A separate memory block: memory changes arrive as pull requests and are flagged in the Inbox.
- Charts. The weekly summary (runs finished, runs merged, model spend) is one line in the Runs header.
- Search or a command palette: later.

### Placeholder data until Phases 2–3

The shell and home are built before issues, pull requests and runs exist. Until each entity lands:

- Placeholder data comes from one source, separate from models, and is visibly labelled in the UI as preview data.
- No models, tables or migrations are created for Issue, PullRequest, AgentRun or RunEvent ahead of their phase.
- Each block switches to real data in the phase that introduces it, and its placeholder is deleted then. Repositories, tokens and model keys use real data as soon as they exist.

## First milestone: Meelting hosts itself

The milestone is done when Meelting's own repository lives in Meelting and a real issue ends in a pull request generated by an agent and merged there. It is the shortest path to the success criterion, because from then on every improvement is developed inside the product.

In scope:

- Users, authentication and personal access tokens.
- Repositories with push and clone over HTTPS, a file browser, commit history and diffs.
- Minimal issues, and pull requests between branches of the same repository, with comments and merge.
- One agent (Claude Code headless) that takes an issue, works in a sandbox and opens an explained pull request.
- Live agent activity and human approval before the pull request is opened.
- Project Memory v0: `.project/` read by the agent at the start of every task.
- The navigation sidebar and the Inbox home described above.

Out of scope:

- SSH, forks, organizations, fine-grained permissions, built-in CI/CD, notifications.
- Own agent runtime, parallel agents and specialized agents.
- Public instance.
- Merge strategies other than merge commit, and conflict resolution in the UI.
- Cancelling a running run, retrying a rejected run with feedback, and a command palette.

Safeguard: a push mirror to GitHub while the forge matures, so a bug cannot put the project's own repository at risk.

## First milestone roadmap

Five sequential phases; each ends with something usable. No dates: weekly hours are intentionally not fixed.

### Phase 0 — Foundations

Done when: `docker compose up` starts the app from a clean checkout.

- [x] Agree on technical conventions with David (defined in `CLAUDE.md`)
- [x] Rails 8 app with Hotwire, PostgreSQL and Solid Queue (initial scaffold committed)
- [x] Users with the Rails 8 authentication generator, plus personal access tokens
- [x] Docker Compose for self-hosting
- [x] Public repository with AGPL-3.0 license, README, contribution guide and DCO

### Phase 1 — Minimal Git hosting

Done when: Meelting's repository lives in Meelting, with a push mirror to GitHub.

- [x] Navigation sidebar and Inbox home with placeholder data (see "Authenticated home and navigation")
- [ ] Create bare repositories on disk
- [ ] Push and clone over HTTPS with a token (`git http-backend`)
- [ ] File browser with syntax highlighting, commit history and per-commit diff
- [ ] Configurable push mirror to GitHub

### Phase 2 — Issues and pull requests

Done when: a pull request is merged without leaving Meelting.

- [ ] Issues with title, description, state and comments
- [ ] Pull requests between branches of the same repository, with diff and general comments
- [ ] Merge commit from the UI (`git merge-tree` + `git commit-tree`, per-repository lock)
- [ ] Inbox "Merge" items and the empty-state "Assign an issue" link use real data

### Phase 3 — First agent: issue to pull request

Done when: a real Meelting issue ends in a pull request generated by the agent and merged.

- [ ] Decide sandbox network policy and how the BYOK key enters the sandbox (blocks the rest of the phase)
- [ ] Docker sandbox per run: clones the repository into a new branch, no host secrets
- [ ] Claude Code headless inside the sandbox, using the user's API key
- [ ] Live activity with Turbo Streams: what it reads, what it edits, which commands it runs
- [ ] Human approval before the pull request is opened
- [ ] Pull request with objective, summary of changes, decisions, tests run and risks
- [ ] Inbox "Approve" and "Failed" items, "In progress" and sidebar counts use real data, updated live

### Phase 4 — Project Memory v0

Done when: the agent cites the decision or convention it followed in a pull request.

- [ ] `.project/` format defined and documented
- [ ] `AGENTS.md` generated block, and a `CLAUDE.md` that imports it
- [ ] Indexing in PostgreSQL through a post-receive hook
- [ ] The agent proposes changes to `.project/` inside its pull requests

## After the first milestone

The order is reviewed at the end of each stage, based on daily use.

1. **Open agents.** Own Ruby runtime with RubyLLM (any provider, including local models), a Codex adapter, and per-agent permissions: which repositories, which tools, what requires approval.
2. **Multi-agent collaboration.** Specialized agents (reviewer, security, QA, documentation), parallel runs, task dependencies, and shared discoveries through memory.
3. **Complete forge.** SSH (first candidate for a Go service), forks, organizations, fine-grained permissions and CI.
4. **Public instance.** Sign-up, per-user compute limits and protection against sandbox abuse.
5. **Ecosystem.** Extensions, third-party agents and shared workflows.

## Non-goals

- A complete GitHub clone before agents work.
- Another GitHub with a chat window.
- A code editor competing with Cursor.
- A model wrapper or a prompt marketplace without real project integration.
- Agents without control or traceability.
- A platform tied to a single AI provider.
- Anything that forces users to abandon Git, or measures success by the amount of code generated.

## Success criteria

The experiment works if Meelting is used daily to develop itself and at least one of David's own projects (for example Veta or Voorder). The second project is a post-milestone goal: the first milestone only requires Meelting to host itself. The evaluation date is not set; a reasonable proposal is 3 months after Phase 0 ends.

Signals to measure from the first agent onward:

| Signal | What it tells us |
| --- | --- |
| Share of issues assigned to an agent that end in a merged pull request | Whether the agent does useful work |
| Review and correction time per agent pull request | Whether delegating saves time or only moves it |
| How often the agent follows or cites `.project/` | Whether memory changes the outcome |
| AI cost per completed task | Whether the flow is sustainable with BYOK |
| Days per week spent developing inside Meelting | The success criterion itself |

If David is still developing outside Meelting on the evaluation date, that is the experiment's answer.

## Risks and open questions

| Risk | Mitigation |
| --- | --- |
| Getting stuck rebuilding the forge and never reaching agents | Closed scope for the first milestone; everything outside "In scope" waits |
| An agent runs arbitrary code inside the sandbox | One container per run, no host secrets, only the user's API key, CPU, memory and time limits; define the network policy before Phase 3 |
| Prompt injection through issue text, comments or repository files makes the agent run harmful commands or leak the API key; the approval gate only covers opening the PR, not what runs in the sandbox | Treat issue and repo content as untrusted; limit sandbox egress; use a low-limit key; consider a key-injecting proxy (open decision) |
| The public instance attracts abuse (mining, spam) and compute cost that BYOK does not cover | Launch it after the milestone, with controlled sign-up and per-user limits |
| GitHub and GitLab close the memory gap before Meelting matures | In-repo memory stays useful even then, because any agent can read it |
| Limited maintainer time (solo side project) with no fixed hours or evaluation date | Closed milestone scope; set an evaluation date once pace is known, otherwise the experiment is never judged |
| Claude Code headless output format changes and breaks RunEvents | Isolate parsing in one adapter; pin the CLI version in the sandbox image |
| Placeholder data outlives its phase and hides missing features | One labelled source, deleted in the phase that brings the real data (see "Placeholder data until Phases 2–3") |

Open questions:

- [ ] Evaluation date for the experiment
- [ ] Sandbox network policy
- [ ] How the BYOK key enters the sandbox (env var vs proxy)
- [ ] How the sandbox authenticates to clone from Meelting without host secrets (short-lived scoped token?)
- [ ] How a post-receive hook reaches Solid Queue (HTTP callback vs `bin/rails runner`)
- [ ] Exact `.project/` format (frontmatter, decision record fields)
- [ ] Which UI locales ship (all copy goes through i18n per `CLAUDE.md`)
- [ ] Whether a running run can be cancelled before the first milestone ends
