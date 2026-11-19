# Tasks and worktrees (Phase 4)

> **Status:** product decisions approved (D27–D48 in [decisions.md](decisions.md)). Phase 4A (repositories, tasks, agent assignment, creation from rooms) is built and verified. Phase 4B (Git environments) is **implemented and verified within the scope tested** on macOS, Ubuntu and Windows (Rust tests), with limits recorded in [§7.1](#71-verification-evidence): no E2E, no real-storage check on runners, no real process-kill test, and a pipeline that is not fully green. 4C (review, drift warnings and E2E) is approved in scope but not started. Platforms: macOS, Windows and Linux.
>
> In this document ✅ means *implemented*. What has been *verified*, and on which platform, is recorded only in §7.1.

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

### 3.4 `task_worktrees` (4B ✅ schema)

Migrations `add_id_repository_id_index_to_tasks` (unique `tasks (id, repository_id)`, needed as the parent key below) and `create_task_worktrees`.

| Column | Type | Rule |
|---|---|---|
| `id` | `TEXT` | PK, UUID v7 |
| `workspace_id` | `TEXT` | `NOT NULL` → `workspaces` `ON DELETE CASCADE` |
| `task_id`, `repository_id` | `TEXT` | `NOT NULL`; `(task_id, repository_id)` → `tasks (id, repository_id)` and `(repository_id, workspace_id)` → `repositories (id, workspace_id)`, both `NO ACTION` |
| `path` | `TEXT` | `NOT NULL`, unique among all rows (removed ones included); absolute path of the worktree |
| `branch_name` | `TEXT` | `NOT NULL`, unique among all rows; 10–120 characters, starts with `meelting/`, only `a-z`, `0-9`, `/` and `-` |
| `base_ref` | `TEXT` | `NOT NULL`; the base branch chosen when preparing |
| `base_commit_sha` | `TEXT` | `NOT NULL`; 40 or 64 lowercase hexadecimal characters; the immutable starting commit |
| `status` | `TEXT` | `preparing`, `ready`, `failed`, `missing`, `removing`, `removed` |
| `error_code` | `TEXT` | Nullable; 1–64 characters of `a-z` and `_`; required when `failed`, allowed only when `failed` or `removed` (a discarded failure keeps its reason); the list of codes is a Rust enum |
| `created_at`, `updated_at` | `TEXT` | `NOT NULL`; `updated_at >= created_at` |
| `removed_at` | `TEXT` | Set if and only if `removed`; `>= created_at` |
| `trashed_at` | `TEXT` | Nullable; when the environment entered the trash (migration `add_trashed_at_to_task_worktrees`) |
| `trash_path` | `TEXT` | Nullable, unique when set (index `task_worktrees_trash_path`); current folder inside `.trash/`; set if and only if `trashed_at` is (migration `add_trash_path_to_task_worktrees`) |

Indexes: unique partial `task_worktrees_active_task_id (task_id) WHERE status <> 'removed'` (at most one active environment per task), `(task_id, repository_id)` and `(repository_id, workspace_id)` for the foreign keys.

**What the foreign keys guarantee.** The first key makes the worktree use the repository of its task, and the second makes that repository belong to the worktree's workspace. Since `tasks` already requires its repository to belong to the task's workspace, task, repository and worktree always share one workspace. Because the first key points at `(id, repository_id)`, a task cannot change or clear its repository while any worktree row (removed ones included) references it, so rule C is enforced by SQLite. The repository row has no update path and cannot be deleted while a worktree references it, so the history keeps the original `git_common_dir`.

**Deletion.** A task cannot be deleted while any worktree row references it (`NO ACTION`). To delete a task, Rust checks in one `BEGIN IMMEDIATE` transaction that no row is active, deletes the `removed` rows (their copies go in cascade) and then the task. A repository cannot be unregistered while tasks or worktrees reference it. Deleting a workspace removes tasks, repositories, worktree rows and copies in the same statement; `NO ACTION` is checked at the end of the statement, so the cascade succeeds. `RESTRICT` is not used because it is checked immediately and would break that cascade depending on the order rows are deleted.

### 3.5 `task_worktree_file_copies` (4B ✅ schema)

Migration `create_task_worktree_file_copies`. One row per file the user copied into a worktree; contents are never stored.

| Column | Type | Rule |
|---|---|---|
| `task_worktree_id` | `TEXT` | `NOT NULL` → `task_worktrees` `ON DELETE CASCADE` |
| `relative_path` | `TEXT` | `NOT NULL`, 1–1,024 characters, `/` separators, no leading `/`, no `\`, no `:`, no empty, `.` or `..` segment and no `.git` segment in any letter case |
| `size_bytes` | `INTEGER` | `NOT NULL`, 0–10,485,760 (10 MiB) |
| `copied_at` | `TEXT` | `NOT NULL` |

PK `(task_worktree_id, relative_path)`. Workspace isolation comes from the parent worktree. The path checks are a safety net: the real validation happens in Rust before copying (BR-06). Rows are kept as history when the worktree is removed and disappear only when the worktree row is deleted together with its task.

### 3.6 Settings (4B ✅ schema)

Migration `add_worktrees_root_to_profile`: `profile.worktrees_root` (nullable `TEXT`, not empty), global per installation. Without a value, `~/Meelting/worktrees` (the home folder of each OS). Changing it only affects new worktrees; existing ones are never moved.

## 4. Business rules

### BR-01. Registering repositories (4A ✅)

The user picks a folder with the native dialog, opened from Rust (`tauri-plugin-dialog`; the webview has no file permissions). Meelting checks with read-only Git commands, never through a shell:

- Git is available and the folder is inside a working tree.
- It is not a bare repository and not a linked worktree (only the main working tree of a repository is accepted, so a managed worktree cannot be registered as another repository).
- It has at least one commit.
- It has no submodules (`.gitmodules` in `HEAD`).
- Its folder and its `git_common_dir` are on fixed local storage (BR-12, added in 4B).
- It is not already registered in the workspace (same `git_common_dir`).

Registering never runs `clone`, `init`, `pull` or writes any file.

### BR-02. Creating tasks (4A ✅)

A task can be created without a repository and without agents. With a repository, the user picks the base branch among its local branches; the default branch (`origin/HEAD` when it points to an existing local branch, otherwise the current branch of the main working tree) is suggested. A task is saved as `pending`, and saving never prepares an environment.

### BR-03. Creating from rooms (4A ✅)

Two entry points open the same task form: «Create task» in the room call controls (origin: the room) and «Turn into task» on a message (origin: the room and the message, with the message prefilled as the description). Agents may suggest tasks in their replies, but only the user creates them. There is no automatic interpretation of messages.

### BR-04. Several agents (4A ✅)

The form selects any number of agents of the workspace, with search. Assigning or unassigning changes nothing else: no status change, no run, no permission. Deleting an agent (when the existing rules allow it) removes its assignments in cascade.

### BR-05. Preparing the environment (4B ✅)

«Prepare environment» is an explicit action in the task detail, available when the task has a repository and a base branch, is neither completed nor canceled and has no active worktree. The procedure:

1. **Checks before writing anything** (read-only Git, no transaction open): the repository folder still holds the registered repository (same `git_common_dir`, main working tree; otherwise `repository_moved`); the repository, its `git_common_dir` and the worktree root are on fixed local storage (BR-12); the base branch resolves to an exact commit (`rev-parse --verify --end-of-options refs/heads/<base>^{commit}`); Git is 2.40 or later; the base commit has no `.gitmodules`; and no file of the base commit has a `filter` attribute whose driver has `clean`, `smudge` or `process` configured, read with `git check-attr --source=<sha>` so repository, global and system configuration count (D45; Git LFS is rejected this way).
2. **Folders:** the root is created if missing and canonicalized; it must not be inside or contain the repository or its `git_common_dir`. The repository folder `<root>/<repo slug>--<last 12 hex of the repository id>` is created if missing and must be a real folder, not a symbolic link. The worktree folder `<repo folder>/<title slug>-<last 12 hex of the worktree id>` must not exist. The branch is `meelting/<title slug>-<last 12 hex of the worktree id>` (D41).
3. **Intent:** in one `BEGIN IMMEDIATE` transaction the task is read again (same repository and base branch, still open) and the `preparing` row is inserted with path, branch, base branch and commit. The partial unique index rejects a second active environment, so concurrent requests create one environment.
4. **Git, under the repository lock:** if the folder or the branch already exists the row becomes `failed` (`checkout_failed`); otherwise `git worktree add --no-track -b <branch> <folder> <sha>` runs with hooks and fsmonitor disabled.
5. **Verification:** the new worktree's top level is the folder, its `git_common_dir` is the repository's, `HEAD` is exactly the base commit and the branch is the expected one. Then the row becomes `ready`; otherwise `failed` (`verification_failed`), and nothing that Git created is deleted.

Uncommitted changes of the original repository are never copied; the UI says that the environment starts from the last commit of the base branch. The original working directory, its index and its checked-out branch are not touched. Preparation is not a security sandbox. After preparation the branch may receive legitimate commits: `base_commit_sha` stays the fixed starting point while `HEAD` moves, and a moved `HEAD` never marks the environment as damaged.

### BR-06. Copying local files (4B ✅)

**Purpose.** Some files a project needs are not in Git (`.env`, local settings, certificates for development). The user can copy them from the original repository into a `ready` environment, whatever the task status (D48). `task_worktree_file_copies` records which files were copied, their size and when, so the environment detail can list them; contents are never stored.

**Selection.** «Copy local files» opens the native multiple-file dialog from Rust, starting in the repository folder. The webview never sends a path: Rust keeps the selection in memory under a single-use identifier tied to the task, and confirming sends only that identifier and whether the user accepted the secret warning.

**Rules for each file** (checked in the preview and again when copying):

- It is a regular file, not a folder and not a symbolic link (hidden files such as `.env` are allowed).
- Its real path is inside the repository (the parent folder is canonicalized, so a folder link pointing outside is rejected) and no segment is `.git` in any letter case.
- It is ignored or untracked: a file tracked by Git is rejected (`git ls-files --cached -- ':(literal)<path>'`), because it already comes with the commit.
- Its name has no `\` or `:` and the path fits 1,024 characters.
- At most 10 MiB per file and 50 MiB per operation (constants in Rust, not settings).
- Nothing in the environment is ever overwritten: a file that already exists there is skipped.

**Preview and confirmation.** The dialog lists every selected file with its relative path, size and, when it cannot be copied, the reason. Files whose name suggests secrets (`.env*`, `id_*`, `*.pem`, `*.key`, `*.p12`, `*.pfx`, `*secret*`, `*credential*`, `*token*`, `.npmrc`, `.netrc`…) are marked. If any of them would be copied, the dialog explains that a copied file is an ordinary file in the environment folder, that Meelting does not hide it and that future agents with tools could read it, and the copy button stays disabled until the user ticks the confirmation. The core also refuses the copy without that confirmation (`secrets_not_confirmed`).

**Copying, under the repository lock (BR-13):** the repository identity, the storage of the repository and the worktree, and the worktree identity are checked again, and so is every file. On macOS and Linux each file is opened with `openat` from a descriptor of the repository folder, one path segment at a time, with `O_NOFOLLOW` at every step, and the opened handle must be a regular file; the destination is created the same way from the worktree folder, creating missing folders with `mkdirat` and the file with `O_CREAT | O_EXCL | O_NOFOLLOW` and the source permissions. A symbolic link swapped in between the check and the copy is refused on the handle itself. On Windows every folder is checked for reparse points and the file is opened with `FILE_FLAG_OPEN_REPARSE_POINT`, refusing a handle that is a reparse point, and the destination is created with `create_new`. At most 10 MiB are read (more fails the copy), the destination is flushed, and a partial file that Meelting created is deleted. Each file gets its own result: copied, skipped (with its reason) or failed. A failure never removes the worktree.

**Secrets and agents.** Copying does not authorize any agent or provider to read the contents, but Meelting does not prevent it either: read isolation for agents belongs to the tool execution of Phase 5, which must treat copied files (at least the ones marked as secrets) as protected. Secret contents never reach the UI, events, logs or errors: only paths, sizes and dates leave Rust.

**Cleanup.** Copied files stay in the environment. Because they are ignored or untracked, they block its removal (BR-10) until the user deletes them; the removal dialog lists them (D44).

### BR-07. Base branch drift (4C, not built)

`base_commit_sha` is immutable and is stored by 4B, but **nothing compares it with the current base branch yet**: no code reads the base branch after preparation and the UI shows no warning. Reporting the drift is part of 4C (D35).

Intended behavior: if the base branch moves, the environment is not updated and the UI warns («main has moved since this environment was prepared»). No rebase, merge or reset. If the base branch is deleted, the worktree keeps its identity and the comparison is reported as unavailable.

### BR-08. Reviewing changes (4C)

Working branch, base branch and commit, Git status, files added/modified/deleted, commits since the base commit, staged and unstaged changes, untracked files and ignored files relevant for preservation, and a read-only textual diff with size and rendering limits. Binary files are shown as metadata.

### BR-09. Completing a task (4A ✅)

Completing sets `completed` and `completed_at`. It never commits, merges, pushes, deletes branches or worktrees, finishes rooms or starts agents. Tasks without a repository can be completed. Reopening is explicit and leaves any worktree untouched.

### BR-10. Removing environments (4B ✅)

Removal is manual and only applies to a `ready` environment. «Remove environment» first runs a check and shows its result; confirming runs the whole check again, so the decision always uses the real state. Both steps hold the repository lock (BR-13).

**Check** (nothing is changed):

1. The repository still holds the registered repository (`repository_moved` otherwise) and the repository, its `git_common_dir` and the worktree folder are on fixed local storage.
2. **Identity:** the folder exists, `git worktree list --porcelain -z` of the repository registers it on the expected branch, its top level is the folder, its `git_common_dir` is the repository's, and `HEAD` points at the branch `meelting/…` that Meelting created. Any difference keeps the environment (`environment_identity_mismatch`). `HEAD` itself may have moved with commits; that is allowed.
3. **Local data:** `git status --porcelain=v2 -z --untracked-files=all --ignored=matching` in the worktree. Any entry blocks removal: modified or staged tracked files, conflicts, untracked files and **ignored files** (for example a copied `.env` or a build folder). This step is required because `git worktree remove` without `--force` still deletes ignored files.
4. **Git operations in progress:** `MERGE_HEAD`, `rebase-merge`, `rebase-apply`, `CHERRY_PICK_HEAD`, `REVERT_HEAD`, `BISECT_LOG` or `index.lock`, resolved with `git rev-parse --git-path` inside the worktree.
5. **Commits only in the branch:** `git rev-list --count <base commit>..refs/heads/<branch>`. They are not uncommitted changes and do not block removal.

If step 3 or 4 finds anything, the UI lists every file with its kind (modified, conflict, not tracked, ignored) or the operation in progress, explains that the files would be lost, and keeps the environment. Meelting never deletes those files for the user (D44).

**Removal** (after confirming, with the check repeated): the row becomes `removing`, `git worktree remove <folder>` runs **without `--force`**, and Meelting verifies that the folder is gone and Git no longer registers it. Then the row becomes `removed`. If Git refuses, the row goes back to `ready`; if the result is unclear, it becomes `failed` (`removal_interrupted`). In every case the error is explained and nothing else is touched. **The branch is never deleted**, with or without commits: the confirmation says that the branch is kept and, if it has commits that are not in the base branch, how many. Commits are never deleted.

### BR-11. Changing the root folder (4B ✅)

Chosen with the native dialog in Settings › Environments, opened from Rust. Before saving, the folder must exist, is canonicalized, must not be inside or contain any registered repository or its `git_common_dir` (in any workspace) and must pass the storage check (BR-12). The screen shows the current folder, whether it is the default or a chosen one, and its storage status. Existing worktrees keep their paths; nothing is moved, copied or repaired, and the new folder only affects new preparations.

### BR-12. Fixed local storage (4B ✅)

Worktrees are only prepared on fixed local storage. The check runs on the repository folder and its `git_common_dir` when registering, on the worktree root when choosing it, and again on the repository, its `git_common_dir`, the root and each worktree before every preparation, copy or removal. For a folder that does not exist yet (the default root), its nearest existing ancestor is checked. Network drives, removable or external volumes and known synchronized folders are rejected; when compatibility cannot be determined, the operation is rejected with an understandable error (`storage_network`, `storage_removable`, `storage_synced`, `storage_unknown`). External disks may be rejected as a consequence (accepted).

| OS | Accepted | Rejected |
|---|---|---|
| macOS | `statfs` reports `MNT_LOCAL`, a local file system type, and `diskutil info -plist` on its mount point reports `Internal` true and `RemovableMedia` and `Ejectable` false | Network file systems (nfs, smbfs, afpfs, webdav…), external or ejectable volumes, `~/Library/Mobile Documents`, `~/Library/CloudStorage` (OneDrive, Dropbox, Google Drive and others), Dropbox folders read from `~/.dropbox/info.json`, and `~/Desktop` and `~/Documents` when iCloud syncs them (if macOS does not let Meelting check this, paths under them are rejected as unknown) |
| Windows | `GetDriveTypeW` reports a fixed drive and `IOCTL_STORAGE_QUERY_PROPERTY` reports a bus other than USB, 1394, SD or MMC and no removable media | UNC paths and mapped or remote drives, removable drives, USB and SD disks, folders under `%OneDrive%`, `%OneDriveCommercial%`, `%OneDriveConsumer%`, `~/iCloudDrive` and the Dropbox folders of `info.json` |
| Linux | `/proc/self/mountinfo` gives a local file system (ext4, xfs, btrfs, f2fs…) and its block device, resolved in `/sys` (through `dm-*` and `md*` slaves), is neither removable nor on USB, MMC or FireWire | Network file systems (nfs, cifs, sshfs, 9p, ceph…), `tmpfs` and unknown types, devices that cannot be resolved (ZFS, loop), `~/Dropbox` and folders from `info.json` |

Every OS also rejects `~/Dropbox`, `~/Google Drive`, `~/OneDrive` and `~/iCloudDrive`. Recognizing synchronized folders is best effort: Meelting does not promise to detect every sync service.

### BR-13. Concurrency (4B ✅)

Operations that touch the worktrees of the same Git repository never run at the same time inside the app: preparing, discarding, recovering at startup and, from steps 5 and 6, removing and copying files take an in-memory lock keyed by `git_common_dir`, so the same repository registered in two workspaces is also serialized. The lock is released when the operation ends, whether it succeeds or fails. No Git command runs while a SQLite transaction is open. The single-instance plugin (D46) prevents a second copy of the app; the lock does not protect against external processes, which Git's own lock files handle. The UI disables an action while it is pending, and the core is the real guarantee (`BEGIN IMMEDIATE` plus the partial unique index).

### BR-14. Recovery (4B ✅)

Git and SQLite are not one atomic transaction: intent is persisted first (`preparing`, `removing`) and reconciled afterwards. At startup, before the window opens, every `preparing`, `ready`, `missing` and `removing` row is checked under its repository lock. An environment is **intact** when its folder exists, `git worktree list` of the repository registers it, and its top level and `git_common_dir` match. It is **gone** when its folder does not exist and Git no longer registers it (or the repository's Git directory no longer exists).

| Stored state | Found | New state |
|---|---|---|
| `preparing` | Intact, on the expected branch and with `HEAD` at the base commit | `ready` |
| `preparing` | Anything else | `failed` (`preparation_interrupted`) |
| `removing` | Gone | `removed` |
| `removing` | Still intact | `ready` |
| `removing` | Anything else | `failed` (`removal_interrupted`) |
| `ready` | Not intact | `missing` |
| `missing` | Intact again | `ready` |

A `ready` environment whose `HEAD` moved or whose branch was switched stays `ready`. Recovery never deletes files, branches, commits or Git registrations and never runs `git worktree prune` or `repair`.

**Discard** (D43): a `failed` or `missing` environment can be discarded only when it is gone. The row becomes `removed` (keeping its error code as history) and a new environment can be prepared. If Git still lists the worktree, discarding is refused (`environment_still_exists`) and the UI explains that the folder must be restored or that the user can prune the registration themselves.

### BR-15. Isolation and security (4B ✅)

- **No shell from the webview:** the Phase 0 `run_in_worktree` spike was removed. No command takes a command line or an arbitrary path; folders and files are chosen with native dialogs opened from Rust, and every worktree command takes only a task id (plus a selection id for copies).
- **Git:** always a child process with structured arguments and no shell; hooks disabled (`core.hooksPath` to `/dev/null` or `NUL`), fsmonitor disabled, `GIT_TERMINAL_PROMPT=0`, `GIT_OPTIONAL_LOCKS=0`, inherited `GIT_DIR`-style and config variables removed, references always fully qualified after `--end-of-options`, pathspecs `:(literal)`, long paths on Windows. Repositories with executable checkout filters or submodules are rejected.
- **Paths:** compared after canonicalization (with the Windows verbatim prefix removed for drive paths), worktree folders never created through symbolic links, identity always checked with Git rather than trusted from the stored path, file copies opened through handles that refuse symbolic links (BR-06).
- **Storage:** fixed local storage only (BR-12).
- **Not a sandbox:** a worktree shares the repository's object store and configuration. No project script, dependency or tool runs in Phase 4.
- **Tauri commands:** the main window can invoke every registered command (Tauri 2 default without an app manifest). An explicit allow-list was evaluated and is recommended as a separate hardening change (see [overview.md](../architecture/overview.md) §7).

### BR-16. Files view and trash (D49)

**Files** (rail) lists the active environments of the workspace with their task, repository, branch, folder and status. Several `ready` environments can be selected, or all at once («Empty all»), and moved to the **Trash** after confirming. Moving checks identity and storage like removal (BR-10 steps 1–2) under the repository lock and runs `git worktree move` into `<repository folder>/.trash/<folder>`; uncommitted changes, ignored and copied files go with it. The row stays `ready` with `trash_path` and `trashed_at` set, cannot be used (copies and removal answer `environment_in_trash`) and still counts as the task's active environment.

**Trash** (rail, above «Collapse») lists the trashed environments with the days left. Both views are a list with a details panel: the list shows task, repository and status per row, and the panel shows the focused environment (the first one by default) with its repository, branch, base and folder, and its own actions (move to trash in Files; restore or delete permanently in Trash). **Restore** moves the folder back to its original path (`worktree_path_conflict` if something is there). **Delete permanently** and **Empty trash** ask for confirmation and then run `git worktree remove --force` and `git branch -D` of the `meelting/…` branch: uncommitted changes, local files and commits only in that branch are lost. The same deletion runs on its own 30 days after `trashed_at`, at startup and every hour while the app is open, in every workspace. A `failed` or `missing` trashed environment is only marked `removed` once it is gone. Batch actions go on after a failure and report each failed task with its error code.

Recovery (BR-14) follows a move that Git did but SQLite did not record: a `ready` or `missing` row that is not intact where it is stored but is intact on the other side of the trash is updated to its real location.

### Deletion rules (D30–D33)

| Entity | Rule |
|---|---|
| Agent | Existing rules unchanged (blocked with messages or runs); when allowed, its `task_agents` rows go in cascade |
| Task | Physical delete allowed only without an active worktree (`preparing`, `ready`, `failed`, `missing`, `removing` count as active); removed-worktree history goes with it (✅ `task_has_environment` otherwise) |
| Repository | Unregistering is allowed only while no task references it; the physical repository is never touched |
| Workspace | There is no workspace deletion today. When it is built (overview §13 decision 10), it must be blocked while the workspace has active worktrees |

A task can change its repository only while it has never had a worktree; once it has history, its repository is fixed (✅ enforced by SQLite through `tasks (id, repository_id)`, and reported as `task_repository_locked`).

## 5. Interface

- **Tasks** (rail): list with filters by status, repository and agent; priority; environment indicator (4B); «New task». Repositories are managed from the same screen.
- **Task form** (shared by Tasks and rooms): title, description, priority, optional repository, base branch when there is one, multi-agent picker.
- **Task detail:** information (description, status, priority, agents, origin) with the allowed status actions, edit and delete; **Environment** (4B) and **Changes** (4C).
- **Files** and **Trash** (rail; Trash above «Collapse»): see BR-16.
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

### Worktrees (4B ✅)

`preparing → ready | failed`; `ready → missing | removing`; `ready` (in the trash, BR-16) `→ removing` on purge; `missing → ready` (found again) or `removed` (discarded once gone); `failed → removed` (discarded once gone); `removing → removed`, `ready` (Git refused) or `failed` (interrupted). Every transition is a guarded update (`WHERE id = ? AND status = ?`). A second active environment is never created to fix an error without reconciling the previous one. A worktree state never changes the task status.

## 7. Acceptance criteria

| ID | Scenario | Expected | Stage |
|---|---|---|---|
| AC01 | Register a valid repository | Linked to the workspace | 4A ✅ |
| AC02 | Register the same repository twice | Duplicate avoided | 4A ✅ |
| AC03 | Register an invalid path or unsupported drive | Clear error, nothing persisted | 4A ✅ (drive: 4B ✅) |
| AC04 | Task without a repository | Saved and can be completed | 4A ✅ |
| AC05 | Assign three agents | Three links kept | 4A ✅ |
| AC06 | Assign an agent of another workspace | Rejected | 4A ✅ |
| AC07 | Create a task from a room message | Task and origin saved | 4A ✅ |
| AC08 | Create a task with a repository | No worktree created | 4A ✅ |
| AC09–AC13 | Prepare, isolation, two tasks of the same repo, duplicate requests, alternative base | See BR-05 | 4B ✅ |
| AC14 | Base branch drift | Warning without rebasing, merging or resetting (BR-07) | 4C (not built) |
| AC15–AC17 | File copies, escaping paths, no overwrite | See BR-06 | 4B ✅ |
| AC18 | Complete a task with a worktree | Environment and branch kept | 4B ✅ |
| AC19–AC21 | Remove with local changes, with an ignored `.env`, clean | Blocked, blocked, removed keeping the branch | 4B ✅ |
| AC22–AC24, AC26–AC28 | Root change, restart during preparation, external move or delete (detected at startup only, BR-14), shared repository across workspaces, unsafe path, executable checkout | See BR-11 to BR-15 | 4B ✅ |
| AC25 | Review code changes | Read-only diff | 4C |
| AC29 | macOS, Windows and Linux | Main flows and recovery validated: E2E on Linux and Windows, automated Rust and frontend tests plus a reproducible manual checklist on macOS (`tauri-driver` has no macOS driver) | 4C |
| AC30 | Multi-agent assignment | No assignment starts an `AgentRun` | 4A ✅ |

File and Git operations are tested with real temporary repositories, including unusual names, spaces, Unicode, symbolic links, existing branches and concurrent operations.

### 7.1 Verification evidence

Implemented (✅ above) is not the same as verified. Evidence is dated and tied to a commit.

**Phase 4B is verified within the scope tested below.** The scope is the Rust tests of 4B on macOS, Ubuntu and Windows plus the frontend checks. It does not cover the points listed under *Not verified*.

**Rust tests.** CI run `44577671082` (2026-11-19, manual dispatch on branch `fix/4b-windows-verification`, commit `76cb3a3`):

| Platform | Result | 4B tests |
|---|---|---|
| macOS | 504 passed, 0 failed, 4 ignored | all pass |
| Ubuntu | 504 passed, 0 failed, 4 ignored | all pass |
| Windows | 499 passed, **2 failed**, 4 ignored | all pass; the 2 failures are outside 4B (below) |

Lint passes (`pnpm build`, frontend tests with 12 files and 200 tests, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`). Bundles for Ubuntu and Windows pass; the macOS bundle fails (below).

**Windows failures of 4B that are now fixed.** On the previous run (`a55329d`: 496 passed, 5 failed) three 4B tests failed. Commit `76cb3a3` fixes them and they pass in run `44577671082`, together with the other worktree, trash, purge, removal, recovery and file-copy tests on Windows:

| Test | Cause | Fix |
|---|---|---|
| `worktree_prepare::tests::interrupted_preparations_are_recovered_or_marked_failed_without_deleting` | The test fixture root kept the `\\?\` prefix that `canonicalize` adds on Windows, and Git rejects it | The fixture root removes the prefix like the implementation does |
| `worktree_prepare::tests::prepares_a_worktree_from_the_exact_commit_without_touching_the_repository` | Same fixture root: `worktree.path` is stripped by the implementation, the fixture was not | Same fixture change; the later assertions now run and pass |
| `system::safe_file::tests::opens_regular_files_below_the_root` | Opening a folder on Windows fails with access denied and surfaced as an I/O error | `safe_file` classifies a folder refused by the OS as «not a regular file». It runs only after the open was refused, so the checks on the opened handle are unchanged |

**Failures outside 4B, still open and not addressed here:**

- Windows (and once, on `4e3401b`, macOS): `intervention_send::tests::failed_or_cancelled_replies_do_not_change_the_proposal` (`ProviderAuthFailed` instead of `Cancelled`).
- Windows: `llm_stream::tests::a_refused_connection_is_an_unreachable_provider` (`Timeout` instead of `ProviderUnavailable`).
- Bundle (macOS): `failed to run actool … Icon.icon`.

The pipeline as a whole is therefore **not green**: the Windows test job and the macOS bundle still fail.

**Frontend.** CI (Node 22, Ubuntu): 12 files and 200 tests pass and the build passes. Locally the same commands ran with an x64 Node 26.3.0 matching the installed native bindings (the default Node on that machine is arm64 and cannot load them) with the same results; the versions differ, so the environments are not claimed to be equivalent. `RoomsPage.test.tsx` (`scrolls to the new message…`) failed once in one local run, so it is flaky. CI runs the frontend checks on Ubuntu only.

**Not verified:**

- End-to-end flows on Linux and Windows (no E2E harness exists) and the macOS manual checklist (AC29).
- The real fixed-storage check (BR-12) on CI runners: the Rust tests inject a permissive checker.
- Interrupted operations on a real process kill: recovery is only simulated.
- Base branch drift (BR-07, AC14) and every 4C capability, which are not built.

## 8. Delivery

- **4A — Domain, repositories and tasks:** migrations `repositories`, `tasks`, `task_agents`; repository registration; task CRUD with statuses, priorities and several agents; creation from rooms; tests.
- **4B — Git environments ✅ (implemented; verified within the scope tested, §7.1):** root setting, storage checks, preparation, base commit, file copies, technical states, concurrency, recovery, safe removal. Step 1 removed the Phase 0 `run_in_worktree` spike (arbitrary shell command in a PTY), `terminal.rs` and `portable-pty` (D27).
- **4C — Review and interface:** environment detail, Git status, read-only diff, drift warnings (BR-07, AC14), error and recovery screens, E2E on Linux and Windows.

## 9. Out of scope

Autonomous tool execution, R2 coordination, cloning remote repositories, automatic commits, merges, pushes or rebases, installing dependencies, setup scripts, Docker or process sandboxes, several active worktrees per task, moving environments between folders, automatic copy or reuse of local files, forced removal of worktrees or branches outside the trash (BR-16), advanced Git LFS and submodule support.
