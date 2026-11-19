import { invoke } from "@tauri-apps/api/core";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Repository } from "../../hooks/useRepositories";
import type { Task } from "../../hooks/useTasks";
import type { Agent } from "../../hooks/useTeams";
import { I18nProvider } from "../../i18n/I18nProvider";
import { TasksPage } from "./TasksPage";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

const ana: Agent = {
  id: "a1",
  team_id: "t1",
  name: "Ana",
  role: "QA Engineer",
  x: 0,
  y: 0,
};
const bob: Agent = {
  id: "a2",
  team_id: "t1",
  name: "Bob",
  role: "Software Developer",
  x: 0,
  y: 0,
};
const app: Repository = { id: "p1", name: "my-app", path: "/code/my-app" };

function task(id: string, title: string, overrides: Partial<Task> = {}): Task {
  return {
    id,
    title,
    description: "",
    status: "pending",
    priority: "medium",
    repository_id: null,
    base_ref: null,
    origin_room_id: null,
    origin_room_name: null,
    origin_message_id: null,
    agent_ids: [],
    completed_at: null,
    created_at: "2026-11-24T10:00:00.000Z",
    updated_at: "2026-11-24T10:00:00.000Z",
    ...overrides,
  };
}

function core(
  handlers: Record<string, (args: Record<string, unknown>) => unknown>,
) {
  invokeMock.mockImplementation(async (command, args) => {
    const handler = handlers[command];
    if (!handler) throw new Error(`unexpected ${command}`);
    return handler((args ?? {}) as Record<string, unknown>);
  });
}

function renderPage() {
  return render(
    <I18nProvider locale="en">
      <TasksPage agents={[ana, bob]} />
    </I18nProvider>,
  );
}

beforeEach(() => {
  invokeMock.mockReset();
});

describe("tasks list", () => {
  it("shows the empty state", async () => {
    core({ list_tasks: () => [], list_repositories: () => [] });
    renderPage();

    expect(await screen.findByText("No tasks yet")).toBeInTheDocument();
  });

  it("filters by status, repository and agent", async () => {
    const user = userEvent.setup();
    core({
      list_tasks: () => [
        task("t1", "Plan", { agent_ids: ["a1"] }),
        task("t2", "Login", {
          repository_id: "p1",
          base_ref: "main",
          status: "in_progress",
          agent_ids: ["a2"],
        }),
      ],
      list_repositories: () => [app],
    });
    renderPage();
    const list = await screen.findByRole("list", { name: "Tasks" });
    expect(within(list).getAllByRole("listitem")).toHaveLength(2);

    await user.selectOptions(screen.getByLabelText("Status"), "in_progress");
    expect(within(list).getByText("Login")).toBeInTheDocument();
    expect(within(list).queryByText("Plan")).not.toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText("Status"), "");
    await user.selectOptions(screen.getByLabelText("Repository"), "none");
    expect(within(list).getByText("Plan")).toBeInTheDocument();
    expect(within(list).queryByText("Login")).not.toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText("Repository"), "");
    await user.selectOptions(screen.getByLabelText("Agent"), "a2");
    expect(within(list).getByText("Login")).toBeInTheDocument();
    expect(within(list).queryByText("Plan")).not.toBeInTheDocument();
  });
});

describe("creating tasks", () => {
  it("creates a task without repository with several agents", async () => {
    const user = userEvent.setup();
    const created = task("t1", "Write the plan", { agent_ids: ["a1", "a2"] });
    const create = vi.fn(() => created);
    core({
      list_tasks: () => [],
      list_repositories: () => [app],
      create_task: create,
    });
    renderPage();

    await user.click(await screen.findByRole("button", { name: "New task" }));
    const dialog = await screen.findByRole("dialog", { name: "New task" });
    await user.type(within(dialog).getByLabelText("Title"), "Write the plan");
    await user.click(within(dialog).getByRole("button", { name: "High" }));
    const options = within(dialog).getByRole("listbox");
    await user.click(within(options).getByText("Ana"));
    await user.click(within(options).getByText("Bob"));
    await user.click(within(dialog).getByRole("button", { name: "Create task" }));

    expect(create).toHaveBeenCalledWith({
      draft: {
        title: "Write the plan",
        description: "",
        priority: "high",
        repository_id: null,
        base_ref: null,
        agent_ids: ["a1", "a2"],
      },
      origin: null,
    });
    expect(
      await screen.findByRole("heading", { name: "Write the plan" }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("suggests the default branch when a repository is picked", async () => {
    const user = userEvent.setup();
    const create = vi.fn((args: Record<string, unknown>) =>
      task("t1", "Login", args.draft as Partial<Task>),
    );
    core({
      list_tasks: () => [],
      list_repositories: () => [app],
      list_repository_branches: () => ({
        branches: ["develop", "main"],
        default_branch: "main",
      }),
      create_task: create,
    });
    renderPage();

    await user.click(await screen.findByRole("button", { name: "New task" }));
    const dialog = await screen.findByRole("dialog", { name: "New task" });
    await user.type(within(dialog).getByLabelText("Title"), "Login");
    await user.selectOptions(within(dialog).getByLabelText("Repository"), "p1");
    const base = within(dialog).getByLabelText("Base branch");
    await waitFor(() => expect(base).toHaveValue("main"));
    await user.selectOptions(base, "develop");
    await user.click(within(dialog).getByRole("button", { name: "Create task" }));

    expect(create).toHaveBeenCalledWith(
      expect.objectContaining({
        draft: expect.objectContaining({
          repository_id: "p1",
          base_ref: "develop",
        }),
      }),
    );
  });

  it("shows core errors in the form", async () => {
    const user = userEvent.setup();
    core({
      list_tasks: () => [],
      list_repositories: () => [],
      create_task: () => {
        throw "agent_not_found";
      },
    });
    renderPage();

    await user.click(await screen.findByRole("button", { name: "New task" }));
    const dialog = await screen.findByRole("dialog", { name: "New task" });
    await user.type(within(dialog).getByLabelText("Title"), "Plan");
    await user.click(within(dialog).getByRole("button", { name: "Create task" }));

    expect(await within(dialog).findByRole("alert")).toHaveTextContent(
      "That agent no longer exists.",
    );
  });
});

describe("task detail", () => {
  it("shows only the allowed status actions and completes the task", async () => {
    const user = userEvent.setup();
    const change = vi.fn(() =>
      task("t1", "Plan", {
        status: "completed",
        completed_at: "2026-11-24T11:00:00.000Z",
      }),
    );
    core({
      list_tasks: () => [
        task("t1", "Plan", {
          origin_room_id: "r1",
          origin_room_name: "Daily",
          origin_message_id: "m1",
          agent_ids: ["a1"],
        }),
      ],
      list_repositories: () => [],
      change_task_status: change,
    });
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Plan/ }));
    expect(
      screen.getByText("A message in room «Daily»"),
    ).toBeInTheDocument();
    const actions = screen.getByRole("navigation", { name: "Change status" });
    expect(
      within(actions)
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["Start", "Block", "Complete", "Cancel task"]);

    await user.click(within(actions).getByRole("button", { name: "Complete" }));

    expect(change).toHaveBeenCalledWith({ taskId: "t1", status: "completed" });
    expect(
      await within(actions).findByRole("button", { name: "Reopen" }),
    ).toBeInTheDocument();
  });

  it("deletes the task after confirming", async () => {
    const user = userEvent.setup();
    const remove = vi.fn(() => []);
    core({
      list_tasks: () => [task("t1", "Plan")],
      list_repositories: () => [],
      delete_task: remove,
    });
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Plan/ }));
    await user.click(screen.getByRole("button", { name: "Delete" }));
    const dialog = screen.getByRole("dialog", { name: "Delete the task?" });
    await user.click(within(dialog).getByRole("button", { name: "Delete" }));

    expect(remove).toHaveBeenCalledWith({ taskId: "t1" });
    expect(await screen.findByText("No tasks yet")).toBeInTheDocument();
  });
});

describe("repositories", () => {
  it("registers through the native picker and shows why one cannot be removed", async () => {
    const user = userEvent.setup();
    core({
      list_tasks: () => [],
      list_repositories: () => [],
      register_repository: () => [app],
      unregister_repository: () => {
        throw "repository_in_use";
      },
    });
    renderPage();

    await user.click(
      await screen.findByRole("button", { name: "Repositories" }),
    );
    const dialog = screen.getByRole("dialog", { name: "Repositories" });
    await user.click(
      within(dialog).getByRole("button", { name: "Add repository" }),
    );
    expect(await within(dialog).findByText("my-app")).toBeInTheDocument();

    await user.click(
      within(dialog).getByRole("button", { name: "Unregister my-app" }),
    );
    expect(await within(dialog).findByRole("alert")).toHaveTextContent(
      "Some tasks use this repository. Change or delete them first.",
    );
  });

  it("keeps the list when the picker is cancelled", async () => {
    const user = userEvent.setup();
    core({
      list_tasks: () => [],
      list_repositories: () => [app],
      register_repository: () => null,
    });
    renderPage();

    await user.click(
      await screen.findByRole("button", { name: "Repositories" }),
    );
    const dialog = screen.getByRole("dialog", { name: "Repositories" });
    await user.click(
      within(dialog).getByRole("button", { name: "Add repository" }),
    );

    expect(within(dialog).getByText("my-app")).toBeInTheDocument();
    expect(within(dialog).queryByRole("alert")).not.toBeInTheDocument();
  });
});
