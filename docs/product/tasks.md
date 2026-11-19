# Tasks and worktrees (Phase 4)

> **Status:** product decisions approved (D27–D39 in [decisions.md](decisions.md)). Phase 4A (repositories, tasks, agent assignment, creation from rooms) is built; 4B (worktrees) and 4C (review and E2E) are approved in scope but not started. Platforms: macOS, Windows and Linux.

## 1. Goal

Organize work in tasks, assign several AI agents to them and prepare isolated Git environments (`git worktree`) without ever touching the working directory of the original repository.

The user can create tasks from the Tasks screen or from a room, assign several agents, link a local repository, prepare a worktree, inspect its changes and manage its lifecycle safely.

Phase 4 **does not** let agents run commands, edit files or perform Git integration operations.

### Four separate capabilities

| Capability | What it is | Where |
|---|---|---|
| **Assigning several agents** | A task records which agents take part in it. Assigning is a label of responsibility: it starts no run, distributes no work and grants no permission | Phase 4A (`task_agents`) |
| **Multi-agent coordination** | Agents taking turns, rounds and discussions driven by a coordinator | Track R stage R2, not Phase 4. R2 may read `task_agents` as a starting point |
| **Worktree preparation** | Creating a branch and a worktree from an exact commit, copying selected local files, reviewing changes and removing clean environments | Phase 4B and 4C |
| **Tool execution** | Agents reading, writing or running commands inside a worktree, with approvals | Phase 5 |

A task is a goal; its agents are participants; the worktree is the prepared file space. Execution and coordination are added later as separate capabilities.

## 2. Domain concepts

- **Workspace:** groups tasks, agents, rooms and repositories. Every operation respects its scope.
- **Repository:** a reference to an existing local Git repository. Meelting records its location and Git identity (`git_common_dir`); it never copies or modifies the code when registering it. The same physical repository can be registered in several workspaces; Git operations are coordinated by its real identity, not by its SQLite id.
- **Task:** a unit of work independent of any technology (implementation, research, documentation, review, planning…). It needs neither Git nor an environment to exist or be completed.
- **Task agent:** the link between a task and an agent of the same workspace. Several per task, no coordinator or hierarchy.
- **Task worktree** (4B): a Git environment of a task and a repository, with its branch, path and starting commit. At most one active per task.
- **Copied file** (4B): a record of a local file the user authorized to copy into the worktree. It never stores contents or secrets.

## 3. Data model

Conventions: `STRICT` tables, `TEXT` UUID v7 ids, `TEXT` ISO-8601 UTC timestamps, composite foreign keys `(id, workspace_id)` for workspace integrity, relation tables without their own id.

### 3.1 `repositories` (4A ✅)

| Column | Type | Rule |
|---|---|---|
| `id` | `TEXT` | PK |
| `workspace_id` | `TEXT` | `NOT NULL` → `workspaces` `ON DELETE CASCADE` |
| `name` | `TEXT` | 1–80 characters; the folder name |
| `path` | `TEXT` | `NOT NULL`; absolute top-level folder as Git reports it |
| `git_common_dir` | `TEXT` | `NOT NULL`; absolute shared Git directory, the repository identity |
| `created_at`, `updated_at` | `TEXT` | `NOT NULL` |

Unique `(workspace_id, git_common_dir)` and `(id, workspace_id)`.

### 3.2 `tasks` (4A ✅)

| Column | Type | Rule |
|---|---|---|
| `id` | `TEXT` | PK |
| `workspace_id` | `TEXT` | `NOT NULL` → `workspaces` `ON DELETE CASCADE` |
| `repository_id` | `TEXT` | Nullable; `(repository_id, workspace_id)` → `repositories (id, workspace_id)` `NO ACTION` |
| `origin_room_id` | `TEXT` | Nullable; `(origin_room_id, workspace_id)` → `rooms (id, workspace_id)` `NO ACTION` |
| `origin_message_id` | `TEXT` | Nullable; `(origin_message_id, origin_room_id)` → `messages (id, room_id)` `NO ACTION`; requires `origin_room_id` |
| `title` | `TEXT` | 1–120 characters |
| `description` | `TEXT` | `NOT NULL`, up to 10,000 characters (`''` when empty) |
| `status` | `TEXT` | `pending`, `in_progress`, `blocked`, `completed`, `canceled` |
| `priority` | `TEXT` | `low`, `medium`, `high` |
| `base_ref` | `TEXT` | Nullable; only with a repository |
| `completed_at` | `TEXT` | Set if and only if `status = 'completed'` |
| `created_at`, `updated_at` | `TEXT` | `NOT NULL` |

Unique `(id, workspace_id)`; index `(workspace_id, updated_at DESC, id DESC)`.

Origin references use `NO ACTION`, not `SET NULL`: a `SET NULL` on a composite key would also null `workspace_id`. Rooms are never deleted (they are finished, D25) and messages are immutable; both disappear only when their workspace is deleted, which also deletes its tasks in the same statement. The composite keys guarantee that the origin room belongs to the task's workspace and that the origin message belongs to that room.

### 3.3 `task_agents` (4A ✅)

`task_id`, `agent_id`, `workspace_id`, `created_at`; PK `(task_id, agent_id)`; `(task_id, workspace_id)` → `tasks (id, workspace_id)` and `(agent_id, workspace_id)` → `agents (id, workspace_id)`, both `ON DELETE CASCADE`. Parent keys are backed by `UNIQUE (id, workspace_id)` on `tasks` and the unique index `agents_id_workspace_id`.

### 3.4 `task_worktrees` and `task_worktree_file_copies` (4B, not created)

The full definition (PK, foreign keys, constraints and indexes) is presented for approval before 4B starts. Approved direction: `id`, `task_id`, `repository_id`, `path`, `branch_name`, `base_ref`, `base_commit_sha`, `status` (`preparing`, `ready`, `failed`, `missing`, `removing`, `removed`), `error_code`, timestamps and `removed_at`; at most one non-`removed` row per task (partial unique index); copies keyed by `(task_worktree_id, relative_path)` with `size_bytes` and `copied_at`, only for successful copies.

### 3.5 Settings (4B)

`profile.worktrees_root` (nullable `TEXT`): global per installation. Without a value, `~/Meelting/worktrees` (the home folder of each OS). Changing it only affects new worktrees; existing ones are never moved.

## 4. Business rules

### BR-01. Registering repositories (4A ✅)

The user picks a folder with the native dialog, opened from Rust (`tauri-plugin-dialog`; the webview has no file permissions). Meelting checks with read-only Git commands, never through a shell:

- Git is available and the folder is inside a working tree.
- It is not a bare repository and not a linked worktree (only the main working tree of a repository is accepted, so a managed worktree cannot be registered as another repository).
- It has at least one commit.
- It has no submodules (`.gitmodules` in `HEAD`).
- It is not already registered in the workspace (same `git_common_dir`).

Registering never runs `clone`, `init`, `pull` or writes any file. The check of fixed local storage (BR-12) arrives with 4B and then applies both when registering and before every preparation.

### BR-02. Creating tasks (4A ✅)

A task can be created without a repository and without agents. With a repository, the user picks the base branch among its local branches; the default branch (`origin/HEAD` when it points to an existing local branch, otherwise the current branch of the main working tree) is suggested. A task is saved as `pending`, and saving never prepares an environment.

### BR-03. Creating from rooms (4A ✅)

Two entry points open the same task form: «Create task» in the room call controls (origin: the room) and «Turn into task» on a message (origin: the room and the message, with the message prefilled as the description). Agents may suggest tasks in their replies, but only the user creates them. There is no automatic interpretation of messages.

### BR-04. Several agents (4A ✅)

The form selects any number of agents of the workspace, with search. Assigning or unassigning changes nothing else: no status change, no run, no permission. Deleting an agent (when the existing rules allow it) removes its assignments in cascade.

### BR-05. Preparing the environment (4B)

«Prepare environment» is available when the task has a valid repository, is neither completed nor canceled, has no active worktree, its base branch resolves to a commit and the destination is safe. Meelting resolves the exact SHA first and creates the worktree from it, with a unique branch (`meelting/task-<short id>-<slug>`) under the configured root (`<root>/<repo>--<short id>/task-<short id>-<slug>`). Uncommitted changes of the original repository are not copied; the UI explains that the environment starts from committed history. Hooks are disabled; repositories that need executable checkout filters (including Git LFS) are rejected with a diagnosis. Preparation is not a security sandbox.

### BR-06. Copying local files (4B)

After preparing, the user can pick individual ignored or untracked files of the original repository (hidden files such as `.env` allowed, with a warning). Not allowed: folders, symbolic links, Git internals, paths outside the repository, silent overwrites. Limits: 10 MiB per file and 50 MiB per operation (constants). A preview with names, paths, sizes and warnings requires explicit confirmation; per-file results are shown and a failure never removes the worktree. Copying does not authorize any agent or provider to read the contents.

### BR-07. Base branch drift (4B)

`base_commit_sha` is immutable. If the base branch moves, the environment is not updated and the UI warns («main has moved since this environment was prepared»). No rebase, merge or reset. If the base branch is deleted, the worktree keeps its identity and the comparison is reported as unavailable.

### BR-08. Reviewing changes (4C)

Working branch, base branch and commit, Git status, files added/modified/deleted, commits since the base commit, staged and unstaged changes, untracked files and ignored files relevant for preservation, and a read-only textual diff with size and rendering limits. Binary files are shown as metadata.

### BR-09. Completing a task (4A ✅)

Completing sets `completed` and `completed_at`. It never commits, merges, pushes, deletes branches or worktrees, finishes rooms or starts agents. Tasks without a repository can be completed. Reopening is explicit and leaves any worktree untouched.

### BR-10. Removing environments (4B)

Manual, re-checking the real state first. Blocked while there are tracked changes (staged or unstaged), untracked files, ignored files to preserve (for example a copied `.env`), files copied or generated by the user, Git operations in progress, or identity or location inconsistencies. A clean worktree is removed with `git worktree remove` after confirmation, **never with `--force`**. Commits only in the branch are not uncommitted changes: the UI warns, keeps the branch and removes only the clean worktree.

### BR-11. Changing the root folder (4B)

Chosen with the native dialog in Settings › Environments, validated before saving. Existing worktrees keep their paths; nothing is moved, copied or repaired.

### BR-12. Fixed local storage (4B)

Worktrees are only prepared on fixed local storage, checked for both the original repository and the worktree root. Network drives, removable volumes and known synchronized folders (iCloud Drive, OneDrive, Dropbox, Google Drive) are rejected. When compatibility cannot be determined, the operation is rejected with an understandable error. External disks may be rejected as a consequence (accepted).

### BR-13. Concurrency (4B)

Operations that change the shared metadata of the same Git repository (create, remove, repair) never run at the same time, coordinated by `git_common_dir` even across workspaces. The UI blocks duplicate requests; locks are released on success or failure.

### BR-14. Recovery (4B)

Git and SQLite are not one atomic transaction: intent is persisted first (`preparing`, `removing`) and reconciled afterwards. At startup, transient worktrees are checked for interrupted preparations, missing environments, incomplete records, worktrees present in Git but not finished in SQLite and environments moved or changed externally. Unknown files or Git records are never deleted automatically; an operation that cannot be recovered safely is marked `failed` with a review action.

### BR-15. Isolation and security

All file operations run in Rust behind specific commands; the webview has no general command interface. Paths are validated against their real identity (symbolic links, normalization, changes between check and use). Git runs as a child process without a shell. No project script, dependency or tool runs in Phase 4.

### Deletion rules (D30–D33)

| Entity | Rule |
|---|---|
| Agent | Existing rules unchanged (blocked with messages or runs); when allowed, its `task_agents` rows go in cascade |
| Task | Physical delete allowed only without an active worktree (`preparing`, `ready`, `failed`, `missing`, `removing` count as active); removed-worktree history goes with it. In 4A tasks have no worktrees yet |
| Repository | Unregistering is allowed only while no task references it; the physical repository is never touched |
| Workspace | There is no workspace deletion today. When it is built (overview §13 decision 10), it must be blocked while the workspace has active worktrees |

A task can change its repository only while it has never had a worktree; once it has history, its repository is fixed (enforced in 4B, when `task_worktrees` exists).

## 5. Interface

- **Tasks** (rail): list with filters by status, repository and agent; priority; environment indicator (4B); «New task». Repositories are managed from the same screen.
- **Task form** (shared by Tasks and rooms): title, description, priority, optional repository, base branch when there is one, multi-agent picker.
- **Task detail:** information (description, status, priority, agents, origin) with the allowed status actions, edit and delete; **Environment** (4B) and **Changes** (4C).
- **Settings › Environments** (4B): configured folder, native picker, validation state, effect on new worktrees only.

## 6. State machines

### Tasks (4A ✅, validated in the domain)

| From | To |
|---|---|
| `pending` | `in_progress`, `blocked`, `completed`, `canceled` |
| `in_progress` | `pending`, `blocked`, `completed`, `canceled` |
| `blocked` | `pending`, `in_progress`, `canceled` |
| `completed` | `in_progress` (explicit reopen) |
| `canceled` | `pending` (explicit reopen) |

### Worktrees (4B)

`preparing → ready | failed`; `ready → missing | removing`; `failed` → controlled recovery or new preparation once no conflicting resource remains; `missing` → recovery or manual review; `removing → removed` or a recoverable error. A second active environment is never created to fix an error without reconciling the previous one. A worktree state never changes the task status.

## 7. Acceptance criteria

| ID | Scenario | Expected | Stage |
|---|---|---|---|
| AC01 | Register a valid repository | Linked to the workspace | 4A |
| AC02 | Register the same repository twice | Duplicate avoided | 4A |
| AC03 | Register an invalid path or unsupported drive | Clear error, nothing persisted | 4A (drive: 4B) |
| AC04 | Task without a repository | Saved and can be completed | 4A |
| AC05 | Assign three agents | Three links kept | 4A |
| AC06 | Assign an agent of another workspace | Rejected | 4A |
| AC07 | Create a task from a room message | Task and origin saved | 4A |
| AC08 | Create a task with a repository | No worktree created | 4A |
| AC09–AC17 | Prepare, isolation, two tasks of the same repo, duplicate requests, alternative base, drift, file copies, escaping paths, no overwrite | See BR-05 to BR-07 | 4B |
| AC18 | Complete a task with a worktree | Environment and branch kept | 4B |
| AC19–AC21 | Remove with local changes, with an ignored `.env`, clean | Blocked, blocked, removed keeping the branch | 4B |
| AC22–AC24, AC26–AC28 | Root change, restart during preparation, external move or delete, shared repository across workspaces, unsafe path, executable checkout | See BR-11 to BR-15 | 4B |
| AC25 | Review code changes | Read-only diff | 4C |
| AC29 | macOS, Windows and Linux | Main flows and recovery validated: E2E on Linux and Windows, automated Rust and frontend tests plus a reproducible manual checklist on macOS (`tauri-driver` has no macOS driver) | 4C |
| AC30 | Multi-agent assignment | No assignment starts an `AgentRun` | 4A |

File and Git operations are tested with real temporary repositories, including unusual names, spaces, Unicode, symbolic links, existing branches and concurrent operations.

## 8. Delivery

- **4A — Domain, repositories and tasks:** migrations `repositories`, `tasks`, `task_agents`; repository registration; task CRUD with statuses, priorities and several agents; creation from rooms; tests.
- **4B — Git environments:** root setting, storage checks, preparation, base commit, file copies, technical states, concurrency, recovery, safe removal. Step 1 removed the Phase 0 `run_in_worktree` spike (arbitrary shell command in a PTY), `terminal.rs` and `portable-pty` (D27).
- **4C — Review and interface:** environment detail, Git status, read-only diff, drift warnings, error and recovery screens, E2E on Linux and Windows.

## 9. Out of scope

Autonomous tool execution, R2 coordination, cloning remote repositories, automatic commits, merges, pushes or rebases, installing dependencies, setup scripts, Docker or process sandboxes, several active worktrees per task, moving environments between folders, automatic copy or reuse of local files, forced removal of worktrees or branches, advanced Git LFS and submodule support.
