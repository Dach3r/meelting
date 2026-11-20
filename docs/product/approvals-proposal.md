# `approvals` table and `run_command` limits (5B proposal)

> **Proposal, not approved.** No migration exists. Nothing here may be created until the product owner approves the schema explicitly.

## Context
- D53: an approval is its own table; the `tool_calls` row stays `running` while it waits; a denial ends it `failed` with `approval_denied`. `tool_calls` itself does not change.
- Product decision (2026-11-19): `run_command` is **approve once**, **approve that exact command for the task**, or **deny**, showing the exact command. `git_commit` needs an approval; never `push`.
- Business rules: «approve for the task» covers the exact command string for that task only and **ends with the task** (tasks can be reopened, so the grant must be ended explicitly). Cancelling a run or finishing the room denies pending approvals.

## Proposed schema

Two migrations, following the one-change-per-file rule:

1. `add_id_task_id_index_to_tool_calls`: a unique index so an approval can reference its call and task together.

```sql
CREATE UNIQUE INDEX tool_calls_id_task_id ON tool_calls (id, task_id);
```

2. `create_approvals`:

```sql
CREATE TABLE approvals (
    id TEXT PRIMARY KEY NOT NULL,
    tool_call_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    tool TEXT NOT NULL CHECK (length(tool) BETWEEN 1 AND 64 AND tool NOT GLOB '*[^a-z_]*'),
    subject TEXT NOT NULL CHECK (length(CAST(subject AS BLOB)) BETWEEN 1 AND 4096),
    status TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'denied', 'cancelled')),
    scope TEXT CHECK (scope IS NULL OR scope IN ('once', 'task', 'granted')),
    granted_by TEXT,
    requested_at TEXT NOT NULL,
    decided_at TEXT,
    revoked_at TEXT,
    CHECK ((status = 'pending') = (decided_at IS NULL)),
    CHECK (decided_at IS NULL OR decided_at >= requested_at),
    CHECK ((status = 'approved') = (scope IS NOT NULL)),
    CHECK ((scope = 'granted') = (granted_by IS NOT NULL)),
    CHECK (scope IS NOT 'task' OR tool = 'run_command'),
    CHECK (revoked_at IS NULL OR (scope = 'task' AND revoked_at >= decided_at)),
    FOREIGN KEY (tool_call_id, task_id) REFERENCES tool_calls (id, task_id) ON DELETE CASCADE,
    FOREIGN KEY (granted_by) REFERENCES approvals (id) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX approvals_tool_call_id ON approvals (tool_call_id);
CREATE INDEX approvals_task_grants ON approvals (task_id, tool, subject)
    WHERE status = 'approved' AND scope = 'task' AND revoked_at IS NULL;
CREATE INDEX approvals_pending ON approvals (id) WHERE status = 'pending';
CREATE INDEX approvals_granted_by ON approvals (granted_by) WHERE granted_by IS NOT NULL;
```

| Column | Meaning |
|---|---|
| `tool_call_id` | The audited call that waits (one approval per call). Run, room and workspace come through `tool_calls`, which already ties them with composite keys |
| `task_id` | Denormalized from the call, tied to it by the composite key; needed to look up task grants without a join |
| `tool` | `run_command` or `git_commit`, checked by format only (like `tool_calls.tool`; the catalog lives in code) |
| `subject` | Exactly what the supervisor approves: the command line for `run_command`, the commit message for `git_commit`. Grants match it byte for byte |
| `status` | `pending` → `approved`, `denied` (by the supervisor) or `cancelled` (run cancelled or timed out, room finished, agent removed, app restarted) |
| `scope` | Only when approved: `once` (this call), `task` (this call and later identical commands of the task), `granted` (approved automatically by an earlier `task` grant) |
| `granted_by` | The `task` approval that covered a `granted` call, for the audit trail |
| `revoked_at` | When a `task` grant ended: set when the task moves to `completed` or `canceled`, so a reopened task starts without grants |

**Flow.** The loop writes the `tool_calls` row `running`, then (unless a live grant matches) an `approvals` row `pending`, emits an event and waits, cancellable, inside the run deadline. Approved: the tool runs and the call closes as usual. Denied: the call ends `failed` / `approval_denied` and the model is told so. Cancelled: the call ends `cancelled` with the run. At startup, every `pending` approval is closed `cancelled` (its run is already expired by the existing recovery).

**What is stored.** `subject` holds the command or commit message, which the supervisor saw and approved; that is the audit's purpose. A command may contain a secret typed by the model; it is shown to the supervisor anyway, and it is already in `tool_calls.arguments` (at most 4096 bytes). Command output is never stored.

**Alternatives considered.** (a) Approval columns on `tool_calls`: rejected by D53, and it would need a rebuild. (b) A separate `command_grants` table: one more table for one more lookup; the partial index covers it. (c) Ending grants by comparing dates with the task status: fragile across reopen/close cycles; `revoked_at` is explicit.

## `run_command` limits (recommendation)

| Limit | Value | Note |
|---|---|---|
| Captured stdout | 64 KiB | As requested; kept in memory for the live terminal (5C), never stored |
| Captured stderr | 64 KiB | Same |
| Sent to the model | 32 KiB per call | The run budget is 128 KiB of accumulated output (D54): 128 KiB from a single command would end the run. Head 8 KiB + tail 24 KiB of the combined output with `[truncated: N bytes omitted]` in between, because test failures usually appear at the end |
| Default timeout | 60 s | As requested |
| Maximum timeout | 300 s | As requested; the model may ask for 1–300 s. Also bounded by the run deadline (15 min cloud, 45 min local) |
| Approval wait | inside the run deadline | No separate timer; the run deadline cancels a pending approval |

No new configuration: these are engine constants like D54.

**Execution (no schema impact).** The command runs in the worktree root through the platform shell (`/bin/sh -c` on Unix, `cmd.exe /d /s /c` on Windows) with a sanitized environment (no provider keys, no `SSH_AUTH_SOCK`, no `GIT_*` variables), its own process group (Unix) or a Job Object with kill-on-close (Windows), stdin closed, and the whole tree killed on timeout, cancellation or drop. Unlike the Guard, it cannot limit the process count: running builds and tests needs child processes. The app states that execution is not isolated (no Docker until Phase 6); the approval is the barrier.

## Open questions for the product owner
1. Approve the two migrations above as written (names, columns, types, constraints, indexes)?
2. Do the 32 KiB sent to the model and the 60/300 s timeouts work for you?
3. Should removing an agent from the room also cancel its pending approvals? Recommended: yes (its runs are already cancelled).
