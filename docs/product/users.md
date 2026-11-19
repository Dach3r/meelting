# Users and core experience

## Target users

**Initial audience:** software developers, technical freelancers and small teams who use several digital tools and want AI to work on their code while they decide whether it is sent to a third-party service.

**Future audience:** startups, Product Managers, development teams, digital agencies.

**Main MVP user:** a developer or technical founder who wants to delegate code tasks to AI agents, keep control of their machine and supervise results from a single interface, even offline.

## Core experience

### 1. First launch (planned)

There is no sign-up, login or account. When the app is opened for the first time:

1. Choose the language (`es` / `en`).
2. Create the first workspace: name required, maximum 80 characters; names can repeat. The profile and the workspace are saved together at the end.
3. The app opens **Teams**: org chart with "You" at the root and the "General" team.

Completion criterion: the first launch ends when `profile.current_workspace_id` points to an existing workspace. If the user closes the app midway through the flow, nothing is saved and they start over from the beginning when they reopen it.

Rules:

- A local profile can have several workspaces and switch between them from the rail.
- The button is disabled while processing, to avoid duplicates.
- Out of scope: accounts, invitations, additional members, sync and integrations.

It must be fast and avoid unnecessary configuration.

### 2. Settings

A page with the categories on the left and the settings of the chosen category on the right, each as a row with its control. Today: **General** (language `es` / `en`) and **Appearance** (theme). Each new setting is one more row within its category; a new category is added to the side list. Later, **workspace** (rename; same rules as when creating it). Docker, shell and local provider are added with their phases. Exporting and deleting local data lives here (Phase 7). Deleting a workspace is an open decision (see [../architecture/overview.md](../architecture/overview.md#13-open-decisions)).

### 3. Main dashboard

See [virtual-office.md](virtual-office.md#dashboard).

### 4. Work room with agents

The central, differentiating experience. The user selects or starts a room and interacts with specialized agents. See [virtual-office.md](virtual-office.md).

### 5. Task on a repository (planned, Phase 4–5)

From a room, the user creates a task by choosing an agent, a repository folder and a base branch. The agent works in an isolated `git worktree`; the user approves the commands that require permission and reviews the diff at the end.
