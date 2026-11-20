# Tools and approvals (Phase 5)

> **Proposal.** The 5A schema and limits are approved (D53, D54); 5A stage A (engine, `list_dir`) is built and gated off, the rest is not yet authorized. Provider choice, approval granularity and commit inclusion were chosen by the product owner on 2026-11-19; everything else here is a recommendation. Nothing in this document approves a schema change: each migration needs its own proposal and explicit approval.

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

## Approved for 5A (D53, D54)
- `tool_calls` audit table, separate from `messages`, with the run, room, workspace, task and environment tied by composite foreign keys. Results are never persisted, only their size and whether they were truncated.
- Per run: 20 iterations, 40 tool calls and 128 KiB of accumulated output; per call: 16 KiB for file reads, 8 KiB for searches and listings, 32 KiB for diffs. Exhausting a budget ends the run with an explicit `error_kind`.
- Tool support is declared in `llm.yml` for cloud models and detected for local ones (`supported`, `unsupported`, `unknown`).

## 5A status (stages A to E built, 2026-11-20; 5A stays open, see the end of this section)

**Built:** the tool loop over Chat Completions inside `AgentRun`, the path jail, the `tool_calls` audit, room → task → environment resolution and five tools: `list_dir`, `read_file`, `search`, `git_status` and `git_diff`. Local tool-support detection (stage D) and the stage E integration are built. Not done: verification on Windows (the tests exist and run in CI, they have not been run), closing BR-17 and opening the production switch.

**Resolution rule.** Tools are offered only when the room is linked to exactly one open task (not `completed` or `canceled`) that the agent is assigned to, and that task has a `ready`, untrashed environment whose identity and storage checks pass. Zero candidates: plain text, nothing is said. Several candidates: the model is told the room is ambiguous and that no tools exist; nothing is chosen by order. One candidate without a ready environment, or with a model that cannot use tools: the model is told so. In every case the model is instructed not to claim it read, changed or ran anything.

**Gate: nothing is enabled in production.** Four conditions are kept apart and all must hold: (1) the **model capability** (declared in `llm.yml` for cloud, detected for local, below); (2) the **environment and authorization** (one eligible task, agent assigned, ready environment, revalidated before every call); (3) the **safety of each tool** (path jail, Guard, limits); (4) the **engine switch** `Engine::tools_enabled`, which production never sets because tools must stay closed until D55 is built and BR-17 and Windows are closed. On top of that, cloud models need the explicit cloud-tools consent (a second engine flag that production never sets; no UI exists to grant it and 5A adds none: reading worktree files sends them to the provider and a name denylist is not enough). `supported` grants no permission by itself: a supported local model is still closed while the engine switch is closed. Granting consent and opening the switch each need their own proposal.

**Local capability detection (stage D).** The state is `supported`, `unsupported` or `unknown` (D54); `unknown` and `unsupported` never enable tools, nothing is inferred from a model's name, architecture or family, no tool is run to find out, and nothing switches model or provider. Only the configured local endpoint is contacted (no redirects, 1 s connect and 3 s total timeouts, 1 MiB body cap); any failure, timeout or malformed answer is `unknown`. Nothing is persisted: the capability is read again for each run (only when the room has one eligible task with a ready environment and the engine switch is open, so closed or task-less rooms make no request) and for each availability check, so a changed or replaced model is noticed immediately.
- **Ollama:** `POST /api/show` with the model; `capabilities` containing `tools` is `supported`, a non-empty list without it is `unsupported`; a missing, empty or malformed list (older Ollama), an unknown model (404) or any error is `unknown`.
- **LM Studio:** `GET /api/v1/models`; the entry whose `key` (or `selected_variant`) is the model and its `capabilities.trained_for_tool_use` (`true` is `supported`, `false` is `unsupported`, missing is `unknown`). That flag is native tool support only: LM Studio's generic prompt-based tool calling is deliberately not relied on, so `false` means "no native support", not "tools can never work". A missing v1 API (older LM Studio), an unlisted model or any error is `unknown`; no older API is tried because its shape cannot be verified here.
- **Verified vs declared:** the parsers and the HTTP behavior are tested against mocks built from each runtime's documented response. They have not been checked against a real Ollama or LM Studio in this repository (an opt-in ignored test reads a real runtime when `MEELTING_LOCAL_PROVIDER`, `MEELTING_LOCAL_BASE_URL` and `MEELTING_LOCAL_MODEL` are set), and a `supported` flag is the runtime's claim about the model, not a measured quality of its tool use.
- **Reporting:** `check_agent_availability` adds `tools` (the same three states) to every agent: from `llm.yml` for cloud models (declared, not a permission), from detection for installed local models, and `unknown` otherwise. The interface does not use it yet.

**Loop rules.** Per run: 20 model iterations, 40 tool calls (a response whose calls do not all fit is refused whole, before running any), 128 KiB accumulated output; per call 8 KiB for `list_dir` and `search`, 16 KiB for `read_file`. Calls run sequentially; every wait is cancellable and bounded by the run deadline. Text the model writes in a turn that also calls tools is kept in the history sent back but is not shown; only the final answer becomes a message, so tool runs have no live token stream.

**Environment checks.** `revalidate` runs before every tool call (file and Git); `resolve` still runs once per run to decide whether tools are offered.

**Retries.** A transient provider failure is retried as before only if no tool has run in the attempt. After the first audited call the run fails with that failure's `error_kind` instead of restarting, so no tool executes twice. A call whose outcome is unknown is never repeated.

**Audit.** A row is written `running` before a tool executes and closed `succeeded`, `failed` (with `error_code`) or `cancelled` exactly once; the result text is never stored. Rows still `running` after a crash are closed `cancelled` at startup.

**File tools (stage B).** `read_file` takes `path` and an optional byte `offset`; it reads at most 16 KiB (a 96-byte reserve holds the note `[truncated: continue with offset N]`), never splits a UTF-8 character, and refuses an offset that falls inside one (`invalid_offset`) or past the end (`offset_out_of_range`). Files with a NUL byte or invalid UTF-8 are `binary_file`; the root, folders and special files are `not_a_file`. `search` takes a literal `query` (at most 256 bytes, no patterns), an optional folder and `ignore_case`, and returns `path:line: text` lines (160 characters each) up to 8 KiB. It walks in-process (no `grep`, no Git): folders sorted by name, symbolic links, `.git`, device names and secret-named entries are skipped, binaries are skipped, and it inspects at most 2,000 files, 1,000 folders, depth 16, 256 KiB per file and 8 MiB in total; hitting any limit ends the result with `[truncated]` instead of an error. Every path is checked by `path_jail` and every open walks from the root with held handles, never following a link at any component (`openat` + `O_NOFOLLOW` on Unix; non-shareable folder handles plus reparse-point checks on Windows); a folder swapped for a link between a listing and a read is refused. Cancellation and the run deadline raise a stop flag that the search checks per folder and per file. Content is never stored: the audit keeps only sizes. **Limit:** the denylist works on names, so a secret inside an ordinary file (for example a key in a config file) can be read and sent to the model; this is why cloud tools need an explicit consent.

**Git tools (stage C).** `git_status` takes no arguments; `git_diff` takes a file `path` (jail rules) and a `source` that is a closed enum (`unstaged`, `staged`, `branch`), never a revision, option or pathspec. Both run only through the guarded executor (`Guard`: hardened command, no hooks, no fsmonitor, no external diff or textconv, attributes pinned to the base commit, executable filters refused, isolated process or refusal), reusing `review_status` and `diff_file` unchanged; the only plain `git` before them is the existing review precondition `verify_identity` on the main repository (see below). Before **every** Git call the environment is revalidated: the room is still linked to exactly the same open task, the agent is still assigned, the environment is the same `ready` untrashed one at the same path (`revalidate`, also applied to the file tools), then the repository lock is taken, identity and storage are verified, the Guard is opened with the branch check, a `review_state` snapshot is taken, and after the query the snapshot and the Git configuration must be unchanged (`environment_changed` / `environment_config_unsafe` otherwise). Any failure to guarantee isolation is a fixed error code (`isolation_unavailable`, `git_unavailable`, `environment_mismatch`, …) that carries no Git output, path or stderr; there is no fallback to ordinary Git. `git_status` hides ignored entries and protected names (it only says how many protected entries were hidden) and is cut at 8 KiB; `git_diff` returns the raw patch of one file cut at 32 KiB on a line boundary with `[truncated]`. Tool call ids must now be 1–128 characters of `[A-Za-z0-9_.:-]` (anything else is `invalid_response`) so a model cannot put arbitrary text in the audit. **Limits:** the diff of a file with an ordinary name can contain secrets, exactly like `read_file`; renames are shown as an addition because only the new path is queried; `verify_identity` runs hardened but not process-isolated `git` against the main repository, which is acceptable only while tools cannot write (it must move onto the Guard before 5B, BR-17); Job Object, `NtResumeProcess` and the atomic snapshot are still pending, so the Guard's process isolation on Windows is not claimed as verified.

**Rejected calls (D55, built).** A call whose arguments are malformed JSON, not an object, over 4096 bytes, or whose tool name is invalid is rejected before running, answered to the model as `error: invalid_arguments`, counted against the call budget and audited as a `failed` / `invalid_arguments` row with no arguments (and no tool name unless it is well formed). It counts as audited, so the run is never restarted over it (the retry rule). The schema change is the migration `rebuild_tool_calls_for_rejected_calls` ([proposal and verification](d55-tool-call-audit-proposal.md)); it exists in `migrations/` and has been run only against temporary test databases, but the app applies embedded migrations at startup, so it takes effect on the first launch: that launch needs explicit authorization.

**Known risks.** DeepSeek models in thinking mode may require their reasoning text to be sent back between tool turns; this is unverified and must be checked before cloud consent is offered. `llm.yml` declares `tools: true` for the two DeepSeek models from the product decision, not from a verified capability test. The identity of the tool root is verified once per run and every call is revalidated against the database (and, for Git, against the repository); there is no atomic snapshot of the directory (BR-17 pending). The Windows held-handle walk has tests for junctions, symbolic links and held handles that have not been run on Windows; its behavior with other reparse points, short names on a real NTFS volume and alternate data streams is not verified there and is not claimed as safe.

## Open for 5B
- Columns of `approvals`, linked to a `tool_calls` row that stays `running` while it waits.
- Independent limits for `run_command` output.

## Dependencies
- Tasks and environments ([tasks.md](tasks.md)), rooms ([rooms.md](rooms.md)), `AgentRun` ([agent-run.md](../architecture/agent-run.md)), Changes review (4C 3B–3C).

**Priority:** Must Have
**Success metric:** share of tool-using runs that end `succeeded`, approvals granted vs denied, and runs where the supervisor opened Changes after the agent's report.

## Stage E: integration and hardening (2026-11-20)

**BR-17 audit, item by item (BR-17 stays open).**
| Item | State |
|---|---|
| `verify_identity` ran `git` against the main repository outside the Guard (no process limit, no timeout, no isolated configuration, child not killed on cancellation) | **Fixed.** It now uses `git_guarded::repository_layout` and `registered_worktrees` (isolated environment, hooks and fsmonitor off, process limit, timeout, kill on drop) and the existing guarded branch check; `worktree list` also needed `-z` for the shared parser, found by the real-runtime test. It also protects the review, trash and purge flows that share it. `Guard::open` also called the non-isolated `git::filters_at` on the main repository (found while classifying BR-17): fixed, `filters_used` now runs `ls-tree` and `check-attr` through the isolated runner (fixed arguments, a validated full commit id, `core.attributesFile` neutralized, 60 s and 32 MiB limits, output beyond the limit is an error, never a partial list), with the paths fed to `check-attr` through a bounded stdin written by a separate task and killed with the process. The non-isolated `system/git.rs` is still used by registration, preparation, removal and the review drift, none of which an agent can trigger. |
| Identity of the environment and concurrent changes | Repository lock, revalidation before every call, Guard branch check and a HEAD/index snapshot compared after each Git call. **Pending:** the snapshot does not see working-tree edits and there is no atomic snapshot of the directory; tools promise consistency per call only. |
| Termination of processes and descendants | Unix: the process limit makes descendants impossible and a dropped query kills the process (tests added). Windows: the Job Object (active-process limit, kill on close) exists; **not run on Windows**. |
| Job Object behavior with launchers and descendants on Windows | **Pending**, unverified. |
| `NtResumeProcess` compatibility | **Pending.** Undocumented but long-standing ntdll export, used to resume the suspended child; no alternative without a spawn API that exposes the thread handle. |
| Short names | **Fixed in the jail:** any component shaped like an 8.3 short name (`ABC~1`, `CREDEN~1.JSO`) is refused and hidden, because it could name a protected file around the name denylist. A real file called `name~1` becomes unreadable by the tools. |
| Elevated-privilege isolation tests | **Pending.** |
| Direct tests of the Tauri command layer | **Pending.** |

**Platform matrix.** Ran here (macOS arm64): the full Rust suite, the frontend suite after fixing the local install (see below), the real Ollama and LM Studio runs. Linux and Windows: not run from here; CI (`.github/workflows/ci.yml`) runs `cargo test` on macOS, Windows and Ubuntu on push, so these tests will run there: junction, symbolic-link and held-handle tests in `safe_file` (Windows only, written and type-checked, never executed), the short-name and reserved-name rules in `path_jail`, the Guard tests (timeouts, config, hooks), and the migration tests. No platform other than macOS is claimed as verified.

**Local runtimes (real, 2026-11-20, nothing downloaded or configured).** Detection against the installed Ollama 0.24.0 and LM Studio: `qwen2.5:0.5b` and `qwen3.6:latest` `supported`, an unknown Ollama model `unknown` (404), LM Studio `google/gemma-4-e4b` and the 12B coder model `supported`, the embedding model and an unlisted name `unknown`; the real answers match the parsers' shapes. Full cycle through the real engine and a real environment (opt-in ignored test `a_real_local_model_runs_the_whole_tool_cycle_in_a_real_environment`): LM Studio `google/gemma-4-e4b` called `list_dir`, `read_file` and `git_status`, all succeeded, and it answered with the first line of `README.md`; Ollama `qwen2.5:0.5b` sent paths the jail refused (`path_outside`) and then `git_status`, which succeeded — the model is too small to use the tools well, the engine behaved correctly. The 36B `qwen3.6` was not run (24 GB). Only these two small models were tried, with a repository without secrets.

**Why 5A stays open.** The switch `Engine::tools_enabled` is off on purpose. Open: BR-17 items above, Windows runs, the cloud consent, applying the migration to real data, the `RoomView` / `RoomsPage` frontend tests that already failed before this work, and the reserved limitations (secrets inside ordinary files, no atomic snapshot, 8.3 names).
