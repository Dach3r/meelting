# Decision log

Centralized log of product and architecture decisions that are not evident from the code. Earlier technical decisions remain in [overview.md §13](../architecture/overview.md#13-open-decisions); new ones are recorded here.

**Statuses:**
- **Approved:** made by the product owner.
- **Pending:** there is a recommendation, but the decision has not been made.
- **Discarded:** evaluated and not pursued.

An approved decision does not mean it is implemented, nor does it approve schema changes: every migration still needs its own proposal and explicit approval.

## Conversational meetings and voice

Context: [voice-meetings.md](voice-meetings.md), [conversation-engine.md](../architecture/conversation-engine.md), [voice-engine.md](../architecture/voice-engine.md).

### Approved (2026-11-18)

| # | Decision | Rationale | Discarded alternatives |
|---|---|---|---|
| D1 | **Text first:** `AgentRun` → text-directed conversation → text multi-agent coordination → voice output → voice input | Context and coordination are validated without depending on audio; voice builds on a stable core | Early spoken MVP (coordination designed with less evidence); meetings only after tasks and tools |
| D2 | **Hybrid coordinator, rules first and optional AI:** rules resolve recipients, mentions, limits and availability; only for "Team" is a suggestion requested from a coordinator model chosen in Settings. Without a model, or if it fails, the fixed round is used | Predictable, testable, works offline and only costs where it adds value | Rules only; 100 % AI coordinator; using the first agent's model (implicit cost and privacy) |
| D3 | **Bounded discussions between agents:** only if the user asks for them, at most 4 automatic interventions, interruptible; when the limit is reached the turn returns to the user | Enables scenario C without open-ended loops or costs | Limit configurable per room (requires persistence); no discussions in the MVP |
| D4 | **Mode per intervention:** the mode (directed, round, team, discussion) is derived from each message's recipient; it is not stored | No new schema; changing mode means choosing another recipient | Per-session toggle; column in `rooms` |
| D5 | **Push-to-talk** in the first version of voice input | Simplifies end of turn, interruption and echo; avoids accidental captures | Continuous listening with VAD; both from the start |
| D6 | **Hybrid online/offline operation:** local or cloud engines and models, configurable by the user and combinable in the same meeting, with no mandatory preference. Local works without a connection; whatever depends on the cloud reports that it is unavailable. States: requires configuration, configured, available, unavailable, requires connection. **No silent fallback:** the provider or model is never switched without consent or an explicitly configured policy | Use the cloud when available without losing local when it is not | Local first with optional cloud; quality first (cloud by default); local only |
| D7 | **Interruption:** if generation finished, the full message is persisted even if the audio was cut off; the "interrupted" mark lives only in memory during the session. If generation was cancelled, there is no message (`failed/cancelled`) | Keeps immutability and the approved `AgentRun` rule without changing the schema | Persisted mark (new column); saving the cancelled partial (changes the `AgentRun` rule); saving only what was heard |
| D8 | **Per-agent voice in a separate table** (`agent_voices`), independent of the LLM provider; without a row, a stable default voice. **The concrete schema is still pending approval** | Separates identity and presentation, supports several engines and does not touch `agents` | Columns in `agents`; not persisting at first |

### Approved (2026-11-18, review for `AgentRun`)

| # | Decision | Rationale | Discarded alternatives |
|---|---|---|---|
| D9 | **Phase 3 providers = the real `llm.yml` catalog:** Anthropic, OpenAI and Groq (cloud); Ollama and LM Studio (local). No DeepSeek, MiniMax or a generic `local` provider in this phase. Each provider uses its own protocol and only the capabilities the catalog declares. Adding a provider later is a new catalog entry and, only if its protocol does not exist yet, a new adapter with real use (formerly P14). **Extended on 2026-11-18 at the user's request:** DeepSeek (cloud, Chat Completions, `deepseek-flash` and `deepseek-v4-pro`) is added as a catalog entry with no new adapter | The catalog is already in the code and in the agents' model picker: existing agents can only have these five providers. Implementing adapters without agents using them would be dead code | Extending the catalog with DeepSeek and MiniMax (adapters without real demand); a single `local` (the catalog and the UI already distinguish Ollama and LM Studio) |
| D10 | **The 4000-character limit applies only to messages written by the user.** Agent responses have no character limit: their length is bounded by the generation parameters each provider supports (`max_output_tokens` or equivalent) and by the catalog's context limits. An incomplete or cut-off response is never saved truncated or silently: the run fails with its `error_kind` (`output_truncated`, `content_filtered`) and no message is created. Messages remain immutable and **the `messages` schema does not change** (formerly P15) | Cutting by characters would break valid responses (code, lists) and hide that text was missing; a model's natural limit is its output tokens. This is what the `messages` `CHECK` and [rooms.md](rooms.md) already implement | 4000 characters for everyone (responses cut or failing for no reason); truncating on save with a mark (requires changing `messages`) |
| D11 | **No room goal for now:** `rooms` does not change. `BuildPrompt` uses the room name and assumes no column that does not exist (formerly P21) | Avoids a schema change without evidence that the name is not enough | `goal` column in `rooms` right away |
| D12 | *(Superseded by D18.)* **An agent's provider and model cannot be changed yet:** `update_agent` keeps its current limitation (name, role, description, instructions and team). It will be evaluated later, together with its effect on existing runs (which in any case keep the provider and model copied when they were created) | Keep the scope; the run does not need it | Enable the change within `AgentRun` |
| D13 | **Voice out of immediate scope:** STT, TTS and voice configuration are not built with `AgentRun`, and `AgentRun` does not depend on audio | Confirms D1; voice consumes persisted messages and does not constrain the execution engine | Designing `AgentRun` with audio hooks |

### Pending

| # | Decision | Recommendation | Blocks |
|---|---|---|---|
| P1 | Is the meeting session (turn plan, last speaker, audio queue, microphone state) persisted? Not to be confused with C1, the `agent_runs` index proposal in [conversation-engine.md §12](../architecture/conversation-engine.md#12-persistence-and-data-model) | No: in-memory state in the core. Does not block `AgentRun` | R2 |
| P2 | When leaving the meeting, do runs in progress continue? | Yes (approved `AgentRun` rule), but with no new turns or audio | R2 |
| P3 | A new intervention while runs from the previous one are active | Cancel them (user priority and cost) | R2 |
| P4 | Is the transcription sent automatically? | Yes if the recipient is resolved; otherwise it stays as a draft | R4 |
| P6 | Does the limit of 4 also apply to "Team"? | Yes: a single budget per intervention | R2 |
| P7 | "Pass" mechanism and `succeeded` without a message | Decide in R2. If approved, it requires rebuilding `agent_runs` (SQLite does not allow changing a `CHECK` without copying the table) | R2 |
| P8 | How a discussion is requested | Explicit "Discussion" recipient + localized list of verbs | R2 |
| P9 | Scope of `settings` (global or per workspace) for the coordinator model and engines | Per workspace for what uses keys; global for devices | R2 |
| P10 | Can agents ask for another agent to take part? | Not in the MVP; later, as a suggestion the user accepts | R2+ |
| P11 | Distribution of voice models | Download with consent, with a catalog and `sha256` | R3 |
| P12 | Audio retention | None: the microphone in memory until transcribed, TTS not saved | R3 |
| P13 | Where audio is played | In the core; confirm with S6 | R3 |
| P16 | Parallel generation with ordered playback (prefetch) | Not at first; measure in R5 | R5 |
| P17 | Push-to-talk shortcut | Only with the window focused, no global shortcut; key to be defined | R4 |
| P18 | Configurable explicit fallback policy | Outside the MVP; design it with its own proposal | Later |
| P19 | Summaries and memory of long meetings | Do not implement; future option of an explicit summary as a visible message | Later |
| P20 | One voice per agent and language (`agent_voices.locale`) | Yes | R3 |
| P22 | Scope of the limit on simultaneous local runs | One slot shared per machine (A9 in [agent-run.md](../architecture/agent-run.md)) | Phase 3 |
| P23 | Does AI coordinator usage count toward daily limits? | Yes | R2 |
| P24 | License of the local Spanish neural TTS (GPL phonemizer) | Legal review before including Piper or Kokoro | R3 |

The decisions approved in the `AgentRun` review supersede P14 (now D9), P15 (D10) and P21 (D11). P5 becomes D19.

### Approved (2026-11-18, review of the `AgentRun` proposal)

| # | Decision | Rationale | Discarded alternatives |
|---|---|---|---|
| D14 | **Architectural direction of `AgentRun`:** states `queued → running → succeeded` or `failed`, with `cancelled` and `expired` inside `failed`, guarded transitions, success (message + `succeeded`) in a single transaction, events after commit, `agent_run:status` and `agent_run:token` with attempt and sequence, unique index `(room_id, client_request_id)`, up to 3 attempts only on transient errors honoring `retry-after`, expiry of `queued` and `running` at startup, idempotent cancellation, one active run per agent and room, local concurrency control and 4 simultaneous in the cloud as a starting point. Out of the first cut: coordination, voice, tools, daily limits and tasks | Ported from Rails and strengthened for local streaming | — |
| D15 | **L1: at most 256 KiB of text per response**, measured in UTF-8 during streaming; when exceeded, `failed/output_truncated` with nothing saved and a translated error. It is an internal safety limit, not the provider's | Protects memory and the database against servers that ignore the token cap; far above any valid response | No technical limit (relying only on the provider) |
| D16 | **`AgentRun` revision 2:**<br>• `client_request_id` mandatory and generated by the client; a repeat returns the original run without calling the provider; different parameters → `client_request_conflict`.<br>• Bounded temporary buffer, isolated per room.<br>• Context frozen on the first attempt.<br>• Idempotent cancellation filtered by workspace.<br>• Maximum duration of 15 min (cloud) and 45 min (local) from the first attempt, including retries and waits, ending in `failed/timeout`.<br>• 1 global local run (Ollama + LM Studio) and 4 in the cloud, **releasing the slot during backoff**, with limits configurable only internally.<br>• Budgets of 24 000 and 8000 characters and 100 messages; if the minimum context does not fit, a controlled error.<br>• Existing keychain for testing, no temporary UI.<br>• Remove the `stream_completion` spike.<br>• Errors `client_request_conflict` and `agent_has_activity`. | Close the contracts before implementing | Keeping the slot during backoff |

### Approved (2026-11-18, final approval of `AgentRun`)

| # | Decision | Rationale | Discarded alternatives |
|---|---|---|---|
| D17 | **Final approval of `AgentRun` (revision 3):**<br>• Full SQL of A1 and A13: the three migrations `add_id_room_id_index_to_messages`, `add_id_room_id_agent_id_index_to_messages` and `create_agent_runs`, with no additional tables or columns; the 45 integrity cases move to `cargo test`.<br>• A1b: permanent `context_too_large`, without calling the provider or trimming instructions or the request.<br>• A2: errors `invalid_client_request_id`, `agent_busy`, `client_request_conflict` and `agent_has_activity`.<br>• A8 and A8b: YAML from §9 with `gpt-6.1-sol` and `gpt-6.1-terra` (128 000, `openai_responses`) and `openai/gpt-oss-120b` (65 536, `chat_completions` with `max_completion_tokens`) instead of the retired models; Anthropic unchanged; local with dynamic detection. Existing agents are not modified and fail with `model_unavailable`.<br>• A8c: internal output caps of 16 000 (cloud) and 2048 (local), and L1 of 256 KiB; the minimum of the documented maximum and the internal cap is requested, adjustable only in the engine's internal configuration.<br>• Order of status events: logical position derived from `status` and `attempts`, with no new column, so a late event does not replace a more recent one with the same attempt.<br>• Retries, concurrency, recovery and cancellation according to revision 3. | Closes the review and authorizes implementation in three deliveries (core and persistence, real execution, interface), testing a real local agent first | Keeping the retired models; output caps equal to the model maximum |
| D18 | **Explicit change of an agent's provider and model** (revises D12; ✅ built) as a **small delivery independent** of the engine: initiated by the user, validated against the catalog, no migration, without modifying existing runs and with no automatic substitutions | Agents with retired models need an explicit way out | Automatic model substitution; doing it within the engine implementation |

### Approved (2026-11-18, R1 — text-directed conversation)

| # | Decision | Rationale | Discarded alternatives |
|---|---|---|---|
| D19 | **Proposed recipient = last agent that responded successfully** (formerly P5): state only in the core's memory, per room; updated only after an agent's response is persisted (a failed or cancelled run does not change it); kept when leaving and returning to the room and lost when Meelting closes; never proposes an agent that no longer participates. Always shown before sending, distinguished from an explicitly chosen recipient, and can be changed or dismissed for an intervention. No columns or tables | Natural follow-ups without choosing the agent on every message, without persisting session state | State only in React (lost when switching rooms); deriving it from `agent_runs` (would survive a restart) |
| D20 | **Several mentions of different agents = ambiguity in R1:** nothing is saved and no run is created until the user chooses a recipient. Rounds arrive with R2 | An intervention produces at most one run in R1 | Launching a round; choosing the first one mentioned |
| D21 | **Three distinct recipient situations:** no explicit selection (mentions, vocative and D19 are resolved); explicit "Nobody" (message with no run: mentions stay as text and D19 does not apply); explicit agent (takes priority, except on conflict with a mention of another agent, which requires clarification) | Choosing "Nobody" is a user decision; an ambiguous value is not reused | Conflict also between "Nobody" and a mention |
| D22 | **The prompt states who each previous user question was addressed to** ("User → QA"), derived from `agent_runs.request_message_id` and from the participants validated by the core, never from labels written by the user. No new columns; same workspace isolation, frozen context and budgets | Prevents an agent from taking a question addressed to another as its own | Not indicating the recipient |
| D23 | **Interventions during a run:** the current behavior is kept: the same busy agent responds `agent_busy`; another agent can run in parallel within the concurrency limits; a message without a recipient is always saved; nothing is cancelled automatically (P3 is left for R2) | No behavior changes in the engine | Cancelling the previous run when intervening |

### Approved (2026-11-18, room participants)

| # | Decision | Rationale | Discarded alternatives |
|---|---|---|---|
| D24 | **Agents join and leave an existing room, as in a real meeting.** Leaving is a soft exit: migration `add_left_at_to_room_agents` with `left_at TEXT NULL` and `CHECK (left_at IS NULL OR left_at >= created_at)`; the participation is not deleted, so their messages and runs are kept under their name. Whoever left does not appear as a participant, receives no instructions and is not a proposed recipient; inviting them again sets `left_at` to `NULL`. If they had an active run in the room, it is cancelled (`cancelled`) in the same transaction | The FK `messages (room_id, agent_id) → room_agents` prevents deleting the participation of someone who has already spoken; deleting their history would break message immutability | Removing only agents without activity (`agent_has_activity`); deleting their messages and runs; blocking the exit while they respond (`agent_busy`) |
| D25 | **Hanging up a room replaces deleting it.** Migration `add_finished_at_to_rooms` with `finished_at TEXT NULL` and `CHECK (finished_at IS NULL OR finished_at >= created_at)`. The `finish_room` command replaces `delete_room`: it sets `finished_at`, cancels active runs (`cancelled`) and keeps messages, participations and runs. The list shows finished rooms at the end, grey and disabled; the core treats them as nonexistent. The room bar uses the hung-up phone icon and asks for confirmation | Know when each call ended without losing the history | Deleting the room; hiding it from the list; allowing it to be reopened read-only |

### Approved (2026-11-18, AI providers screen)

| # | Decision | Rationale | Discarded alternatives |
|---|---|---|---|
| D26 | **"AI providers" in Settings without schema changes.** Keys live only in the keychain (`<workspace_id>/<provider>`); whether a provider is configured is read from there, and local providers (Ollama, LM Studio) use the catalog URL and are shown read only. The generic `save_secret`, `secret_last_four` and `delete_secret` commands are removed: the webview only has `list_provider_keys`, `save_provider_key`, `check_provider_key` and `delete_provider_key`, which take a cloud provider of the catalog and resolve the workspace in the core. A key is saved only if a minimal request (first catalog model, 16 output tokens, catalog `base_url`) succeeds; without a connection or with the provider failing it is not saved, and a failed replacement keeps the previous key. Opening the screen makes no cloud request; the stored key is re-verified only with "Check connection", which never deletes it | Closes Phase 3 with the hybrid model intact (local works offline, cloud reports when it cannot be reached, nothing switches provider) and without letting the webview name arbitrary keychain entries or send a key to a URL of its choice | The `llm_providers_configured` table; an editable local URL; "Save without verifying" when offline; verifying with `GET /models` (does not detect billing or model access); checking every key automatically when the screen opens |

## Tasks and worktrees (Phase 4)

Context: [tasks.md](tasks.md).

### Approved (2026-11-24, Phase 4 specification and technical review)

| # | Decision | Rationale | Discarded alternatives |
|---|---|---|---|
| D27 | **Only local Git repositories, registered without touching them.** The user picks the folder with the native dialog opened from Rust (`tauri-plugin-dialog`, no file permission for the webview). Accepted: main working tree, not bare, with at least one commit, without submodules; identity = absolute `git_common_dir`, unique per workspace. The Phase 0 `run_in_worktree` spike (`worktree_run.rs`, `terminal.rs`) is removed in 4B, where `git.rs` gets the Phase 4 operations | Registering must never change the user's repository; the same repository can live in several workspaces and Git operations are coordinated by its real identity | Cloning remote repositories; bare repositories; registering linked worktrees; a webview file picker |
| D28 | **The repository is optional** and tasks without one can be completed | A task is a goal, not a technology | Requiring a repository |
| D29 | **Several agents per task** (`task_agents`, no coordinator or leader). Assigning starts no run, distributes no work and grants no permission; R2 may use it as a starting point | Assignment is responsibility; coordination (R2) and execution (Phase 5) are separate capabilities | A single `agent_id` on `tasks`; creating runs on assignment |
| D30 | **Deleting an agent removes its assignments in cascade** when the existing rules allow the deletion (blocked with messages or runs, unchanged) | Being assigned is not activity | Blocking the deletion of assigned agents |
| D31 | **A task can be deleted only without an active worktree** (`preparing`, `ready`, `failed`, `missing` and `removing` are active until reconciled); removed-worktree history goes with it. A task can change its repository only while it has never had a worktree | Never leave a worktree on disk without a record | Only cancelling; deleting with active worktrees |
| D32 | **A repository can be unregistered only while no task references it**; the folder is never touched | Keeps task history consistent | Unregistering in cascade |
| D33 | **Workspace deletion, when it exists, is blocked while it has active worktrees** | SQLite cascades must never leave worktrees unmanaged | Deleting worktrees in cascade |
| D34 | **Tasks can be created from Tasks and from a room** («Create task» and «Turn into task» on a message), with the same form; the origin room and message are recorded with composite keys `NO ACTION` (rooms are only finished and messages are immutable; both disappear only with their workspace) | Turning a conversation into work without automatic interpretation; agents only suggest, the user creates | `ON DELETE SET NULL` (it would null `workspace_id` in a composite key); simple keys validated only in services |
| D35 | **The worktree is created only with the explicit «Prepare environment» action**, from the exact SHA of the chosen base branch (the default branch is suggested, another can be chosen); the SHA is kept and drift is reported without rebasing, merging or resetting | Saving a task must not touch disk; reproducible starting point | Preparing on save; following the moving branch |
| D36 | **Completing a task keeps its worktree and branch** (narrowed by D49 for the trash); removal is manual, re-checks the real state (tracked, untracked and ignored files, Git operations in progress) and never uses `--force` | Never lose local work | Removing on completion; forced removal |
| D37 | **Local files are copied only when the user selects and confirms them per task** (individual files, no folders or symbolic links, no overwrite, 10 MiB per file, 50 MiB per operation); contents are never stored | `.env` and similar files are often needed but must not leak or be copied silently | Automatic copy; reusing a list across tasks |
| D38 | **Fixed local storage only**: network drives, removable volumes and known synchronized folders (iCloud Drive, OneDrive, Dropbox, Google Drive) are rejected for both the original repository and the worktree root; if compatibility cannot be determined, the operation is rejected | Git metadata shared by worktrees breaks on network and synced storage | Warning only; allowing external disks |
| D39 | **Worktree root `~/Meelting/worktrees`, configurable in Settings with the native dialog**, global per installation (`profile.worktrees_root`); a change affects only new worktrees, nothing is migrated. E2E runs on Linux and Windows in 4C; macOS uses Rust and frontend tests plus a reproducible manual checklist | One predictable place outside app data; `tauri-driver` has no macOS driver | Per-workspace setting; app data folder; moving existing worktrees |

### Approved (2026-11-25, Phase 4B technical review)

| # | Decision | Rationale | Discarded alternatives |
|---|---|---|---|
| D40 | **Unique index `tasks (id, repository_id)`** as the parent key of `task_worktrees (task_id, repository_id)`, so SQLite forbids changing a task's repository once it has worktree history | Integrity in the database, not only in services | Checking it only in Rust |
| D41 | **Names from the random part of the worktree UUID v7:** branch `meelting/<slug>-<last 12 hex of the worktree id>` and folder `<root>/<repo slug>--<last 12 hex of the repository id>/<slug>-<last 12 hex of the worktree id>`; readable slugs truncated to a safe length. The first characters of a UUID v7 are a timestamp | Branches are kept after removal (D36), so preparing the same task again needs a new name; random suffixes avoid collisions | The task id in the name (collides on re-preparation); the time-based prefix of the UUID |
| D42 | **`error_code` checked by format in SQLite (`[a-z_]`, 1–64) and typed as a Rust enum** | Adding a code does not require rebuilding the table (lesson of P7) | A closed `CHECK` list |
| D43 | **«Discard» marks a `failed` or `missing` environment `removed` without `git worktree prune`**, only after checking that its folder and its Git registration are gone | `prune` cleans every stale worktree of the repository, including the user's own | Pruning automatically |
| D44 | **No automatic deletion of copied files in the MVP** (narrowed by D49: only the trash deletes them); the environment detail lists the files that block removal so the user can delete them | Without a content hash Meelting cannot know whether the user changed them | A «delete copies» action |
| D45 | **Executable checkout filters are rejected:** the effective attributes of the base commit files are read with `git check-attr --source=<sha>` (repository, global and system configuration included); any `filter` whose driver has `clean`, `smudge` or `process` configured (Git LFS included) blocks preparation | A filter runs a program during checkout | Allowing LFS; inspecting only the repository `.gitattributes` |
| D46 | **Single instance** with `tauri-plugin-single-instance`; the in-memory lock per `git_common_dir` only coordinates operations inside the app and is not considered a protection against external processes | Two app instances would bypass the in-memory lock | Relying only on Git's lock files |
| D47 | **Minimal platform dependencies** (`libc` on Unix, `windows-sys` on Windows) for the storage checks; Linux reads `/proc` and `/sys`, macOS also asks `diskutil` as a child process | Checks need OS APIs; no plist or wrapper crates | Higher-level crates |
| D48 | **Copies only into a `ready` worktree, whatever the task status**, chosen with the native dialog opened from Rust | A valid environment may need its files even if the task is blocked or completed | Tying copies to the task status |

### Approved (2026-11-19, Files view and trash)

| # | Decision | Rationale | Discarded alternatives |
|---|---|---|---|
| D49 | **Trash for environments with complete deletion after 30 days.** «Move to trash» runs `git worktree move` into `.trash/` next to the worktree folder (same volume, local files included) and can be undone; deleting from the trash, by hand or 30 days after `trashed_at`, runs `git worktree remove --force` and deletes the `meelting/…` branch with `git branch -D`, commits included. The usual removal (BR-10) stays without `--force` and keeps the branch | The user asked for a complete deletion with a recovery window; the move keeps the Git registration, so restoring is exact | Deleting at once; keeping the branch on purge; a trash outside the worktree root (could cross volumes) |

**Mandatory corrections of the review:** (A) the base commit is fixed but the branch `HEAD` may move with legitimate commits; identity checks use path, Git registration, `git_common_dir` and branch, never `HEAD`; commits and branches are never deleted. (B) Task eligibility and the `preparing` insert happen in one `BEGIN IMMEDIATE` transaction, the per-repository lock also covers copies, and no Git command runs while a SQLite transaction is open. (C) Symbolic-link checks are made on the opened handles with OS-specific mechanisms. (D) A copied secret is not assumed to be hidden from agents: the warning requires confirmation and the real read isolation belongs to Phase 5 tools; secret contents never reach the UI, events, logs or errors. (E) Removal follows a defined procedure without `--force` and never deletes the branch. (F) No command exposes a shell; Git runs with structured arguments and a controlled environment.

### Approved (2026-11-19, Settings)

| # | Decision | Rationale | Discarded alternatives |
|---|---|---|---|
| D50 | **Settings reorganized into a flat list of sections with one card per group of settings, and five new global preferences in `profile`** (`time_format`, `reduce_motion`, `notify_run_succeeded`, `notify_run_failed`, `notify_sound`) added by **one** migration, `add_preferences_to_profile` (an explicit exception to one file per column). **Open at login** is read from and written to the operating system through `tauri-plugin-autostart` with no column; notifications are sent from the core with `tauri-plugin-notification` when a run ends `succeeded` or `failed` and the window is not focused; **Data folder** is opened from the core with `tauri-plugin-opener`. Provider keys stay in their own view (D26). Specification in [settings.md](settings.md) | The structure grows by adding a setting, a group or a section; the OS owns the login item so there is nothing to drift; a notification needs no user mention because mentions go from the user to agents, so «Run failed» replaces «Mentions» | Per-workspace settings; keyboard shortcuts (none exist yet); density (overlaps interface size and touches every component's spacing); quiet hours; debug log (no logging infrastructure); «Reset settings» and «Clear history» (no destructive actions for now); moving provider keys into Settings; one migration per column |

### Risks and dependencies

See the risks in [conversation-engine.md §14](../architecture/conversation-engine.md#14-risks) and [voice-engine.md §12](../architecture/voice-engine.md#12-risks), and the stage dependencies in [roadmap.md](roadmap.md#track-r--conversational-meetings).
