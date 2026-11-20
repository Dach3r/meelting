# Tools and approvals (Phase 5)

> **Proposal**, not approved for implementation. Provider choice, approval granularity and commit inclusion were chosen by the product owner on 2026-11-19; everything else here is a recommendation. Nothing in this document approves a schema change: each migration needs its own proposal and explicit approval.

**Problem:** agents talk in a room but cannot work. The user cannot ask for a code change and get a verifiable result.

**Goal:** from a room, the supervisor asks for a code change; an agent makes it in its task worktree, runs the relevant tests with the supervisor's approval, and reports the result in that same conversation with a link to the diff. No voice, no agent-to-agent coordination beyond what Track R already built.

**User story:** As a supervisor, I want to ask an agent in a room to change code and review the result before anything else happens, so that I can delegate real work without losing control.

## Scope decisions (product owner, 2026-11-19)

| Topic | Choice |
|---|---|
| First providers | **DeepSeek (cloud) and local (Ollama, LM Studio)**, both over Chat Completions. Anthropic and OpenAI Responses follow in 5D |
| `run_command` approval | **Approve once, approve that command for the task, or deny**, showing the exact command |
| `git_commit` | **Included, with approval**; never `push` |

## Functional flow
1. The room has a linked task with a prepared environment and the agent assigned to it.
2. The supervisor asks the agent for a change (directed message, R1).
3. The agent loops: model reply → tool calls → results → model reply, until it answers in text.
4. Read-only tools and edits inside the worktree run without asking. `run_command` and `git_commit` stop and wait for an approval.
5. The room shows each tool call, the approval dialog with the exact command, and live command output.
6. The agent's final message goes to the chat and links to the **Changes** section of the task.

## Tools

| Tool | Approval | Notes |
|---|---|---|
| `read_file`, `list_dir`, `search` | no | Path jail; secret-name denylist on reads |
| `git_status`, `git_diff` | no | Through the guarded Git executor (BR-17) |
| `write_file`, `edit_file` | no | Only inside the worktree; every path canonicalized |
| `run_command` | **yes** | Timeout, output cap, process tree terminated on expiry |
| `git_commit` | **yes** | Task branch only; never `push` |

## Business rules
- The conversation never authorizes tools: only an approval does.
- Path jail: every path is canonicalized and must stay inside the task worktree; symlinks escaping it are refused.
- Sanitized environment: no provider keys, no `SSH_AUTH_SOCK`.
- Without Docker (Phase 6) the app states that execution is **not isolated**; approval is the only barrier.
- «Approve for the task» covers the exact command string for that task only and ends with the task.
- Cancelling a run or finishing the room denies pending approvals and ends running commands.
- Every tool call is recorded for auditing. Secret contents never reach the UI, events or logs.
- A model that cannot do native `tool use` fails with a clear reason; nothing falls back silently to another provider or model.

## Delivery cuts
- **5A — Tool engine, no UI:** tool domain (path jail, definitions, sanitized environment), read-only tools, tool loop inside `AgentRun` over Chat Completions (DeepSeek and local), `tool_calls` table.
- **5B — Write and execute:** `write_file`, `edit_file`, `run_command`, `git_commit`, `approvals` table, timeouts and process cleanup.
- **5C — Room interface:** tool activity in the chat, approval dialog, live terminal, link to Changes, end-to-end check of the goal above.
- **5D — Other providers:** Anthropic Messages and OpenAI Responses `tool use`.

## Risks
- `run_command` runs arbitrary code (tests included); only approval protects the machine without Docker.
- Secrets copied into a worktree (4B) could be read by an agent; the denylist is partial.
- Local `tool use` quality depends on the model; availability checks must say whether the installed model supports tools.
- Pending Windows hardening (Job Object, `NtResumeProcess`) affects 5B.
- The loop can run away: needs a step budget per run and cancellation at every step.

## Open points for approval before 5A
- Exact columns of `tool_calls` (and `approvals` in 5B).
- Step budget per run and output caps per tool.
- How tool activity is shown in the message history (separate rows from `messages`).
- Whether the model must declare tool support in `llm.yml` for cloud and be detected for local.

## Dependencies
- Tasks and environments ([tasks.md](tasks.md)), rooms ([rooms.md](rooms.md)), `AgentRun` ([agent-run.md](../architecture/agent-run.md)), Changes review (4C 3B–3C).

**Priority:** Must Have
**Success metric:** share of tool-using runs that end `succeeded`, approvals granted vs denied, and runs where the supervisor opened Changes after the agent's report.
