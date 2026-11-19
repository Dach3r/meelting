# AI agent catalog

> **Status:** Phase 2 complete. Already available: listing teams and agents, creating teams (name only), deleting empty teams (with confirmation, from the team panel), the catalog of 25 templates (embedded es/en YAML) with search and category filter, each template's detail, hiring with a step-by-step wizard, creating custom agents by choosing the model from `llm.yml` (cloud or on this computer, with search; in "On my computer" the user picks Ollama or LM Studio and the models responding on `localhost` are listed, detected when the step opens, with a warning and a "Detect again" button if the runtime is not running or has no models; the model name is never typed by hand) and an org chart shaped like a board ("You" → teams → agents on a dotted canvas, with cards, collapsing and expanding teams and agents, card selection with handles, free dragging of teams and agents with persisted positions (agents follow their team, cards do not leave the org chart area, lines are SVG and a "Rearrange" button restores the layout), a side panel with the selected agent's data (team, role, provider and model) or the selected team's (a team's panel lists its agents), with edit and delete with confirmation, and floating zoom). Pending: "Hire now with the template data" (requires a default provider and model from `llm.yml`), warning for a provider without a key, minimap, pinning, each agent's status and the initial "General" team. Behavior ported from the Rails product, where it was already implemented.

**Problem:** the workspace starts empty; the user cannot build their team, which is the foundation of the virtual office.
**Goal:** let the user hire agents from a template catalog, customize them and manage them as their workspace's persistent team. Each agent has its own AI provider and model and works when the user directs an instruction to it in a room ([agent-execution.md](agent-execution.md)).
**User stories:**
- As a workspace owner, I want to explore a catalog of specialized agents, to understand which roles I can add.
- As an owner, I want to hire an agent and adjust its name, role, description and instructions.
- As an owner, I want to view, edit and delete my agents.
- As a new user, I want a clear first step in Teams, to know how to get started.

## Scope and exclusions

**Includes**
- Catalog with 25 templates, one per predefined role, as YAML files embedded in the app. A template's category (Product, Engineering, Data, Design and support, Business) comes from its role's group; the catalog is shown as one section per category (icon, name and count) whose cards are laid out in a grid that fills according to the width, and each card carries the name's initial in a neutral circle. The user searches by name, role or description and filters by category; categories with no matches are not shown.
- `agents` table belonging to a workspace, with a copy of the template's values.
- Separate sections: **Agent catalog** and **Teams**.
- Hiring from the catalog, creating custom agents without a template, viewing, editing and deleting.
- Teams as the home screen, with a draggable org chart: teams that group agents under "You" (every agent belongs to a team), free dragging of teams and agents (agents follow their team); an agent's team is changed when editing it, in a table of teams sorted by name, with each team's agents and the current team marked (when changing team its saved position is discarded and it reappears in the new team's automatic layout; saving without changing team keeps it).
- Provider and model picker per agent, including the local providers Ollama and LM Studio.

**Excludes**
- Collaboration between agents, metrics, token usage and statistics on the cards.
- Limit of agents per workspace.
- Automatic change of an agent's content when the language changes.

## Functional flow

1. The user picks "Teams" in the rail.
2. Sees the org chart with "You" at the root, teams and agents; clicking a team highlights it and clicking the root returns to all. "New team" opens a dialog to name it.
3. With a team selected, clicks "Hire agent" and reaches that team's **Agent catalog**.
4. "View details" shows the template's detail (avatar, role, description and instructions by section). "Hire" opens the same step-by-step form as "Create custom agent" (Identity, Engine, Behavior, with a side step indicator, Back and Continue), prefilled with the template's data; the last step confirms with "Hire". In Behavior, each field comes with help text and an example; the description is short and the instructions take the remaining space, both grow with the text, and with a template "Restore template" appears when the instructions were edited.
5. On confirming, the agent is created in the active workspace with the source `template_key` and its profile is shown with a toast.
6. From Teams, each agent can be viewed, edited or deleted (with confirmation). Editing is not a wizard: it opens side tabs (Identity, Team, Instructions; all the same height, and Team uses the same open list with search as Role) that can be jumped to with one click, and each one is an independent form with its own "Save changes" (disabled without changes); the dialog stays open after saving and is closed with "Close".
7. The agent's profile lists the rooms it participates in or an empty state.

**Teams:** created, renamed and deleted (only without agents). Each workspace starts with a "General" team. An agent is moved to another team from its edit dialog.

**Custom agent:** from Teams, "Create custom agent" opens a three-step form (Identity, Model, Instructions) and creates an agent with a null `template_key`. The role is chosen from a closed list of predefined roles (`catalogs/agent_roles.yml`), grouped by area (Product, Engineering, Data, Design and support, Business) in an always-open list with search and keyboard support (arrows, Enter); the core rejects any role not in it, also when hiring from a template.

**Language:** hiring copies the values of the active language; the saved agent does not change if the interface language changes later.

The same template can be hired several times.

## Business rules
- An agent belongs to a single workspace; all queries start from the current workspace.
- Hiring copies the template's values. The agent does not read the YAML again.
- `template_key` is for traceability only: it is set on creation, not changed on edit and does not constrain the role. It is validated against the catalog only on creation.
- An agent can only be in a team of its own workspace; a team with agents cannot be deleted.
- Deleting a workspace deletes its teams and agents.
- An agent with messages or runs cannot be deleted.
- No screen suggests activity that does not exist.

## Data model (SQLite)

Full detail in [../architecture/overview.md](../architecture/overview.md#data-model).

**`agents`:** `id` (UUID v7), `workspace_id`, `team_id` (FK `ON DELETE RESTRICT`), `template_key` (NULL if custom), `name` (1–80), `role` (1–80), `description` (≤ 500), `instructions` (≤ 10 000), `provider` and `model` (non-empty), timestamps. `(team_id, workspace_id)` is a composite foreign key to `teams`.

**`teams`:** `id`, `workspace_id`, `name` (1–80, lowercase slug with hyphens, e.g. "Main Website" → `main-website`; normalized in the form and again in the core), timestamps.

**Validations:** mirrored in the database with `CHECK`; `provider` and `model` are validated against `llm.yml` on creation and when they change (done); an agent whose model left the catalog remains editable and its "Model" tab warns that another one must be chosen.

**Avatar:** agents have no selectable image; `AgentAvatar` draws a circle with the initials (max. 2) of the first two words of the name.

**Templates (no table):** `agent_templates/<key>.yml`, one file per template, embedded in the app and read with `serde_yaml`:

```yaml
key: product_owner
es:
  name: Product Owner
  role: Product Owner
  description: ...
  instructions: ...
en:
  ...
```

Localized content lives only in the YAML; interface text goes in the frontend i18n files. Catalog and agents are independent concepts: the agent only knows the key.

## Tauri commands (planned)

| Command | Effect |
|---|---|
| `list_agent_templates`, `get_agent_template(key)` | Catalog in the active language |
| `list_teams`, `create_team`, `delete_team`, `update_team` | Teams of the current workspace |
| `list_agents`, `get_agent`, `create_agent`, `update_agent`, `delete_agent` | Agents of the current workspace |
| `save_canvas_positions(positions)` | Org chart: saves the offset of teams and agents (ignores unknown nodes) |
| `reset_canvas_positions` | Org chart: deletes the saved positions and returns the teams in their automatic layout |

The frontend does not access SQLite directly; commands are thin and call core services.

## Acceptance criteria
- [ ] "Teams" in the rail opens the org chart and appears active only there.
- [ ] The catalog shows the 25 templates with role and description, and can be searched and filtered by category; its detail shows the instructions.
- [ ] Hiring creates an agent in the active workspace with the edited values and the source `template_key`.
- [ ] The same template can be hired more than once, and a custom agent can be created.
- [ ] Changing the language does not modify already hired agents.
- [ ] `name` and `role` made only of spaces are invalid.
- [x] An agent can be edited (including provider and model, D18 in [decisions.md](decisions.md)) and deleted with confirmation.
- [ ] Each agent uses its provider and model, chosen from a dependent picker fed by `llm.yml`. A provider without a key can be chosen with a warning.
- [ ] An agent from another workspace does not appear and cannot be modified after switching workspaces.
- [ ] Validation errors visible in the form; complete text in `es` and `en`.

## Tests
- Core (`cargo test`): repositories and `CHECK`, validations, `template_key` only on creation, workspace integrity.
- Templates: loading the 5, nonexistent key, `es` and `en`, no duplicate keys.
- Components (Vitest): `AgentRow`, `AgentAvatar`, `AgentTile`, `TeamNode`, `Rail`/`NavItem` with active state.

## Future implications
- **Rooms:** `room_agents` already covers the relationship without changing `agents`.
- **Execution:** status and usage live in `agent_runs`, not in `agents` columns.
- **User templates or marketplace:** the catalog would be migrated to a table without changing `agents`.

**Priority:** Must Have (Phase 2).
**Success metric:** % of workspaces with at least 1 hired agent after first launch; agents per workspace; % of custom agents.
