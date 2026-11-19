# Virtual office

> **Status:** vision; everything is planned. The Rails product already validated room, participants, chat and runs; here they are rebuilt in the desktop app (see [../architecture/overview.md](../architecture/overview.md)).

The main interface is inspired by a video conference room. Instead of people's cameras, it shows AI agents through avatars, activity states and visual surfaces that represent what they are doing.

## Elements

### AI agents

Each agent is a team member with a specific role. Templates planned for v1 (see [agent-catalog.md](agent-catalog.md)):

| Role | Responsibility |
|---|---|
| Product Owner | Analyzes requirements, organizes priorities, defines acceptance criteria. |
| Software Developer | Implements features, reviews code, proposes changes. |
| QA Engineer | Identifies bugs, defines tests, verifies results. |
| DevOps Engineer | Analyzes infrastructure, proposes operations. |
| Project Manager | Coordinates tasks, identifies blockers, keeps track of progress. |

### Work room

Space where the user observes the agents, takes part in conversations and supervises their activity: goal, participants as cards with their live status and persistent conversation as the meeting transcript. The user directs an instruction at an assigned agent and sees its response arrive token by token (see [rooms.md](rooms.md) and [agent-execution.md](agent-execution.md)).

### Screen sharing

Visual surface for inspecting actions, results and artifacts of the work performed. In the desktop product it will be the **task panel**: worktree status, live terminal (`TerminalPanel`) and diff (`DiffViewer`).

**Rule:** the visualization must correspond to real, verifiable activity. Do not simulate actions that never happened.

### Collaborative chat

Channel where the user and agents exchange instructions, results, questions and updates.

### Approval center

Dialog (`ApprovalDialog`) that shows the exact command an agent wants to run. The user approves once, approves for the task or denies. See [agent-execution.md](agent-execution.md#tools-and-approvals).

## Workspace

Organizational space where the user manages their team of agents. It contains:

- Workspace name.
- Teams and agents.
- Work rooms.
- Worktree tasks.
- AI keys and configuration.

There are no additional human members: the app is for a single local user.

## Dashboard

Entry point to the workspace. The home screen is **Teams** (org chart). In the vision it will give access to rooms, show agents, and display recent activity, pending tasks and approvals.

First MVP: only capabilities backed by real features.

## Work room with agents — example flow

User request: *"Add validation to the sign-up form and run the tests."*

1. The user creates a task on their repository; the app creates a branch and a worktree.
2. The Developer reads the code and proposes changes inside the worktree.
3. The agent requests approval to run `npm test`; the user grants it.
4. The user watches progress live and reviews the diff.
5. The user keeps the branch or discards the worktree.

Agents must complete real work, not merely act out a fictional conversation. Task delegation between agents remains out of scope for v1. Coordinated conversation between agents (questions to the team and bounded discussions, always started by the user) and voice are proposed in [voice-meetings.md](voice-meetings.md).
