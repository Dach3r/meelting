# Work rooms

> **Status:** built (Phase 3): rooms, participants, history, sending user messages and, with `AgentRun`, the "Ask" selector, streaming agent responses and each agent's "Thinking…" state ([agent-execution.md](agent-execution.md)). Agents can join and leave an existing room (D24). Pending: the screen sharing panel. Behavior ported from the Rails product.

**Problem:** the user has agents, but there is no place to bring them together.
**Goal:** let the user create rooms, assign agents and converse in a persistent history. The room is the unit of working context: teams organize agents; rooms bring them together to work. Assigned agents respond when the user directs an instruction at them ([agent-execution.md](agent-execution.md)).
**User story:** As the owner of a workspace, I want to create a room, assign agents to it and converse in it, so the working context is ready before the agents run.

## Scope and exclusions

**Includes**
- `rooms` table belonging to `workspaces`, with a required name.
- Agent participation (`room_agents`): an agent can be in several rooms and a room can have several agents, from any team in the same workspace.
- Create, view and finish rooms (hang up the call). Participants are chosen at creation with a search by name or role; chosen ones become chips removed with their cross or with Backspace (when the search is empty); Enter adds the first match. A created room's name is not editable, but its agents do change: they join and leave like in a real meeting (D24). If duplicate agents arrive, the core deduplicates them keeping the order of first appearance, without error.
- A room without agents is allowed.
- Ask the whole team: the «Ask» selector has «The whole team, in turn» («Todo el equipo, por turnos»), and a message that starts with «Team,» or «Equipo,» does the same unless someone is @mentioned. Up to 4 agents answer one at a time in room order, each seeing the earlier replies; an agent with nothing to add shows a quiet «{agent} has nothing to add» line instead of a message. «Stop» (visible while any run is active) cancels the active turn and discards the pending ones; cancelling a single run or sending a new message to an agent does the same (D52).
- Invite and remove agents from an existing room. "Invite agents" opens the same search with the workspace agents not present; after choosing agents and pressing "Invite", a confirmation with the names is requested, and each tile's cross opens a confirmation dialog before removing (they can be invited again). Whoever leaves stops being a participant, but their history is kept under their name; if they were responding, their run is cancelled.
- User and agent messages (`messages`), with persistent history per room. Live delivery uses **Tauri events** emitted by the core. Messages are immutable: no editing or individual deletion.
- Room screen as a video call, full screen (covers the rail and the top bar; "Rooms" goes back to the list): one tile per participant (the user and each agent with their role), the conversation as a transcript in a side panel with the composer (Enter sends, Shift+Enter inserts a line break), the number of agents in the header and a floating control bar with "Invite agents", "Check availability" and "Hang up" (hung-up phone icon). While an agent has an active run in the room, its tile shows "Thinking…" and a ring.
- Participant lobby: room list as a grid of Grafito theme cards, each with name, activity date, overlapping avatars of up to three agents with a counter for the rest and a full-width button to enter.
- Composer with `@` mentions, "Ask" selector ("Based on the message", "Nobody" or an agent) and proposed recipient ([conversation-engine.md](../architecture/conversation-engine.md#r1-built)). Below the instruction appear the agent with its status (queued, thinking, writing or retrying), the live text and "Cancel", or the translated error. The saved response shows provider, model and tokens only in its footer.
- Each message shows the time, and on hover, the full date. If an agent's role matches its name, it is not repeated.

**Excludes**
- Archiving and renaming rooms, editing or deleting messages, history pagination (the last 100 are loaded).
- Additional human members.

## Functional flow
1. The user opens **Rooms** and sees their rooms or an empty state. The list is sorted by recent activity (`rooms.updated_at`, which is the creation time and is updated with each message; ties broken by `id`): the first one is highlighted as a meeting table (one card per participant, the user included, and a preview of its last 3 messages) and the rest appear on a dotted floor plan as rooms with their agents seated (up to 3, with "+N" for the rest) and their last activity, plus a dashed "New room" slot.
2. Creates a room: name and, optionally, agents.
3. On creation they enter directly; they see the participants as tiles and the conversation (last 100 messages) to one side.
4. Writes a message: the core saves it and updates the room's activity in a transaction, returns it and, after the commit, emits `message:created`. The UI adds it from the result or the event without duplicating it (by `id`).
5. Invites agents from the "Invite agents" control or removes one with its tile's cross; both actions ask for confirmation (`add_room_agents`, `remove_room_agent`; both return those present and those who left).
6. Hangs up with confirmation: the room becomes finished (`rooms.finished_at`), its active runs are cancelled and the list shows it at the end, grey and disabled, with the finish date. It cannot be entered again and nothing is deleted.

## Acceptance criteria
- [x] Create and finish rooms in the current workspace.
- [x] Assign agents from any team when creating the room; an agent is not duplicated in a room.
- [x] Invite agents to an existing room and remove them; whoever leaves keeps their history under their name, receives no instructions and their active run is cancelled; inviting them again brings them back.
- [x] An agent from another workspace cannot be assigned, even by tampering with the command arguments.
- [x] A room from another workspace cannot be read or written (core error `room_not_found`).
- [x] The author of a user message is set by the core, never by the command argument.
- [x] Messages persist and are emitted to the UI through `message:created`.
- [x] Empty states, errors and confirmations, in `es` and `en`.

## Business rules
- `name` required, 80 characters max.
- Message `body` required (the core trims leading and trailing whitespace). 4000 characters max for user messages only; agent responses have no character limit and are not truncated: they are bounded by the model's output tokens (approved decision D10, [agent-execution.md](agent-execution.md)). The UI, the core and SQLite count the same way: Unicode code points (`Array.from` in the UI, `chars()` in Rust, `length()` in SQLite) over the trimmed text; a simple emoji counts 1 and one composed with ZWJ counts each code point. If the message exceeds 4000, the composer disables sending and shows the warning, without cutting the text.
- The room and agent of a participation must belong to the same workspace; this is validated in the service and reinforced by composite foreign keys with `workspace_id`.
- An agent can only have messages in rooms where it participates (foreign key `messages (room_id, agent_id) → room_agents`).
- Removing an agent sets `room_agents.left_at` (does not delete the row) and cancels its active run in the room in the same transaction. Only participants without `left_at` appear in the room, the list, availability and recipient resolution; those who left are used to name their messages in the UI and in the prompt history. Removing someone not present fails with `agent_not_found`; inviting someone already present changes nothing.
- Finishing a room does not delete messages, participations or runs; active runs are cancelled (`cancelled`) and any later action on the room returns `room_not_found`. Deleting an agent deletes its participations, unless it has messages or runs: then it cannot be deleted. The core returns its own error `agent_has_activity`.
- All queries start from the current workspace.
- An `agent_ids` with agents from another workspace (or nonexistent ones) fails and changes no participation: assignment is atomic (one SQLite transaction).

## Dependencies
- Agents and teams ([agent-catalog.md](agent-catalog.md)).
- SQLite and Tauri events.

## Planned evolution

✅ With R1, `send_intervention` replaces `send_message` and `send_instruction`: a message without a recipient is saved as before (same validations, `rooms.updated_at` and `message:created`), and one directed at an agent also creates its `AgentRun`. The core resolves the recipient ([conversation-engine.md](../architecture/conversation-engine.md#r1-built)). `list_rooms`, `create_room`, `get_room` and `message:created` do not change; `delete_room` was replaced by `finish_room`.

| Level | Status | What it adds | Document |
|---|---|---|---|
| Current room | ✅ implemented | Room, participants, history, user messages | This document |
| Conversation with `AgentRun` | ✅ Built (Phase 3) | Responses from an agent chosen in "Ask", with streaming and cancellation | [agent-execution.md](agent-execution.md) |
| Directed and multi-agent conversation | Proposed (R1–R2) | Mentions, recipient per intervention, "Team", bounded discussions, in-memory meeting session | [voice-meetings.md](voice-meetings.md) |
| Voice | Proposed (R3–R5) | Per-agent voice, push-to-talk, subtitles | [voice-meetings.md](voice-meetings.md), [voice-engine.md](../architecture/voice-engine.md) |
| Tasks from the room | ✅ Built (Phase 4A) | «Create task» in the call controls and «Turn into task» on a message open the task form; the task keeps the room and message as its origin (`tasks.origin_room_id`, `tasks.origin_message_id`). The same dialog can link an existing task instead (`task_room_links`). Agents may suggest tasks but only the user creates them | [tasks.md](tasks.md) |
| Real work | Planned (Phases 4B–5, R6) | Worktrees for those tasks, screen sharing with real terminal and diff. Messages remain conversation only | [agent-execution.md](agent-execution.md#tools-and-approvals) |

**Related decisions:** agents join and leave with a soft exit (D24); the room has no goal column and the prompt uses its name (D11); agent responses do not have the 4000-character limit (D10). See [decisions.md](decisions.md).

**Priority:** Must Have
**Success metric:** percentage of workspaces that create a room with at least one agent and send a message.
