# Agent tools for local models on macOS (proposal)

> **Approved (D56, option A, 2026-11-20) and built.** No migration and no schema change. Status and evidence at the end of this document.

## Context
- The tool engine is built (5A stages A–E, 5B `write_file` and `edit_file`): `list_dir`, `read_file`, `search`, `git_status`, `git_diff`, `write_file`, `edit_file`. See [tools-and-approvals.md](tools-and-approvals.md).
- Every run is still plain text because the engine switch `Engine::tools_enabled` is never set in production, and cloud models also need a consent flag that has no UI ([gate](tools-and-approvals.md#5a-status-stages-a-to-e-built-2026-11-20-5a-stays-open-see-the-end-of-this-section)).
- Both `Engine::enable_tools` and `Engine::grant_cloud_tools_consent` exist only under `#[cfg(test)]`.

**Problem:** the supervisor can talk to agents and create tasks, but no agent can touch the code of a task, so nothing can be delegated.

**Goal:** the supervisor can turn on tools for **local models only (Ollama, LM Studio), on macOS only**. With tools on, an agent in a room linked to its task can read, search and edit files in the task worktree, and the supervisor reviews the result in **Changes**.

**User story:** As a supervisor on macOS with a local model, I want to turn on agent tools explicitly, so that my agents can work in their task worktree while nothing leaves my machine.

## How to turn it on: alternatives

| | Option | Advantages | Risks |
|---|---|---|---|
| **A** | **Toggle in Settings › Advanced, kept in memory: off at every launch** | No schema change; a visible, explicit opt-in in every session; a forgotten switch closes itself on restart | Has to be turned on again after each restart |
| B | Same toggle persisted in `profile` (new column) | Remembered across launches | Needs a migration with its own approval; a switch left on stays open without the user noticing |
| C | Environment variable at launch (`MEELTING_AGENT_TOOLS=1`) | Smallest change | Not usable from the installed app; invisible in the interface |
| D | Always on for local models on macOS | No friction | No opt-in; contradicts the gate rule that tools never open by default |

**Recommendation: A.** It keeps the four gate conditions separate, adds no column, and the cost (turning it on once per session) is small while the feature is experimental. B can come later, once 5C shows tool activity in the room.

## Functional flow
1. Settings › Advanced shows an **Agent tools** card with a toggle, off at every launch, and a warning: agents with a local model can read and change files in the worktrees of their tasks; they cannot run commands or commit; on other systems it reads «Available on macOS only» and the toggle is disabled.
2. Turning it on or off calls the core, which sets the engine switch. Cloud consent stays off.
3. The supervisor prepares the usual setup: a task with a repository, the agent assigned, a **ready** environment, and the task linked to the room.
4. The supervisor asks the agent for a change in the room. If the agent's local model is detected as `supported`, the run offers the seven tools; otherwise it stays plain text and the model is told why (unchanged rules).
5. The agent answers in the chat when it finishes. The supervisor opens **Changes** in the task to review the diff and can discard the environment if it is wrong.

## Business rules
- **Local only.** Cloud agents stay plain text with the toggle on: the cloud consent flag keeps no command and no UI.
- **macOS only.** The core refuses to turn the switch on elsewhere (`unsupported_platform`), not only the interface. Reason: the Windows held-handle walk is verified only on CI, not on a real NTFS volume, and nothing was run on Linux from here.
- **Off at every launch.** Nothing is persisted.
- **Turning it off** applies to runs that start afterwards; a run already using tools finishes or is stopped with Cancel (existing behavior). Alternative not recommended now: checking the switch before every call, which adds a new error code.
- **Unchanged rules:** one eligible task per room, revalidation before every call, path jail, guarded Git, D54 budgets, `unknown` and `unsupported` capability never enable tools, nothing switches provider or model.
- **Not included:** `run_command`, `git_commit` and approvals (they wait for [approvals-proposal.md](approvals-proposal.md)); tool activity in the chat (5C).

## Risks accepted by this proposal
- **Secrets inside ordinary files** (a key in a config file) can be read by the agent. With a local model they stay on the machine; this is why cloud stays closed.
- **Writes without approval** inside the worktree (the approved rule for edits). The original working tree and anything outside the worktree are never touched; a bad result is discarded with the environment.
- **No atomic snapshot of the directory:** consistency per call only; a concurrent change can make a call fail, never escape the worktree.
- **Elevated privileges not tested:** on Unix the Guard refuses to run as root, so it fails closed.
- **No tool activity in the room until 5C:** while tools run, the agent shows as working with no live text; only the final answer appears.
- **Model quality:** small local models misuse tools (see the real runs in [tools-and-approvals.md](tools-and-approvals.md)); the engine refuses bad paths, but the result may be poor.
- **D55 migration:** `rebuild_tool_calls_for_rejected_calls` is applied at the first launch of a build that contains it. Approving this proposal includes that launch; a copy of `meelting.db` should be made before it.

## Acceptance criteria
- [ ] At every launch the switch is off and the toggle shows off.
- [ ] On macOS, with the toggle on, a `supported` local model in an eligible room uses the tools and its changes appear in **Changes**; checked with a real LM Studio or Ollama model.
- [ ] With the toggle on, a cloud agent is still plain text.
- [ ] Outside macOS the core refuses with `unsupported_platform` and the toggle is disabled.
- [ ] Turning the toggle off makes the next run plain text.
- [ ] Copy in `es` and `en`; Rust and frontend tests; `cargo clippy -- -D warnings`, `cargo fmt --check`, `pnpm build`.

## Changes on approval
- `Engine`: `set_tools_enabled(bool)` usable in production, replacing the test-only `enable_tools`. The cloud consent setter stays test-only.
- A service with the macOS check and two thin commands, `get_agent_tools` and `change_agent_tools`.
- Hook `useAgentTools` and the **Agent tools** card in Settings › Advanced.
- Docs: [tools-and-approvals.md](tools-and-approvals.md), [settings.md](settings.md), [decisions.md](decisions.md) (D56 approved), `CLAUDE.md`.

## Dependencies
- 5A stages A–E and 5B write tools (built), D55 (built, migration pending first launch), tasks and environments, rooms, Changes.

**Priority:** Must Have (first time an agent does real work).
**Success metric:** runs with tools that end `succeeded`, and runs after which the supervisor opened **Changes**.

## Status (2026-11-20)

**Built:** `Engine::set_tools_enabled` (production), `services/agent_tools_change.rs` (macOS check at compile time, returns `enabled` and `available`), commands `get_agent_tools` and `change_agent_tools`, hook `useAgentTools`, the **Agent tools** card in Settings › Advanced › Agents and its `es`/`en` copy. Cloud consent keeps its test-only setter. Local providers point at fixed `http://localhost:` URLs from the embedded catalog (pinned by a catalog test), so tools only reach a runtime on this machine.

**Tests:** the switch (off at start, on and off on macOS, refused elsewhere: that case runs on Windows and Linux CI), the Settings card (turning it on through the core, disabled where unavailable), and two opt-in tests against a real runtime in `tool_environment.rs`.

**Real check (macOS arm64, LM Studio `google/gemma-4-e4b`):** asked to replace the first line of `README.md`, the agent called `read_file`, `edit_file` and `read_file` (all `succeeded`), the run ended `succeeded`, the worktree file starts with `Hello from Ada`, the original repository is unchanged and clean, and the Changes review (`review_task_environment`) lists `README.md` as modified. The read-only opt-in test still passes after it was refactored to share the setup.

**D55 on real data:** the migration was applied by the running development app at 2026-11-20 18:06:59, before the backup could be made; the backup `backups/meelting-2026-11-20-before-agent-tools.db` in the data folder was taken afterwards with `sqlite3 .backup` (integrity `ok`), and the real database passes `integrity_check` and `foreign_key_check`.

**Not done here:** the click-through in the running app (it was checked through the same services the commands call); Windows and Linux (refused by design).

**Later iterations (not blocking this flow):**
- ~~Show tool activity in the room (5C)~~: built by D58.
- Tell the user in the room when tools are off or the model is not `supported` (today only the model is told).
- ~~A direct link to **Changes** from the room~~: «View changes» under the activity (D58).
- Checking the switch before every call, so turning it off also stops a run that is already using tools.
- Persisting the switch (option B), once activity is visible.
