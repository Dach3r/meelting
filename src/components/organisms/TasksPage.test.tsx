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
    worktree: null,
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
    await user.click(
      within(dialog).getByRole("button", { name: "Create task" }),
    );

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
    await user.click(
      within(dialog).getByRole("button", { name: "Create task" }),
    );

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
    await user.click(
      within(dialog).getByRole("button", { name: "Create task" }),
    );

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
    expect(screen.getByText("A message in room «Daily»")).toBeInTheDocument();
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

describe("task environment", () => {
  const linked = task("t1", "Login", {
    repository_id: "p1",
    base_ref: "main",
  });
  const worktree = {
    id: "w1",
    path: "/Users/ana/Meelting/worktrees/my-app--aabbccddeeff/login-5c6d7e8f9a0b",
    branch_name: "meelting/login-5c6d7e8f9a0b",
    base_ref: "main",
    base_commit_sha: "0123456789abcdef0123456789abcdef01234567",
    status: "ready" as const,
    error_code: null,
    copies: [],
    created_at: "2026-11-25T10:00:00.000Z",
    updated_at: "2026-11-25T10:00:00.000Z",
  };

  it("prepares the environment explicitly and shows its branch and base commit", async () => {
    const user = userEvent.setup();
    const prepare = vi.fn(() => ({ ...linked, worktree }));
    core({
      list_tasks: () => [linked],
      list_repositories: () => [app],
      prepare_task_environment: prepare,
    });
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Login/ }));
    const section = within(screen.getByRole("region", { name: "Environment" }));
    expect(
      section.getByText(/starts from the last commit of main/),
    ).toBeInTheDocument();
    await user.click(
      section.getByRole("button", { name: "Prepare environment" }),
    );

    expect(prepare).toHaveBeenCalledWith({ taskId: "t1" });
    expect(
      await section.findByText("meelting/login-5c6d7e8f9a0b"),
    ).toBeInTheDocument();
    expect(section.getByText("main @ 0123456789ab")).toBeInTheDocument();
    expect(section.getByText("Ready")).toBeInTheDocument();
    expect(
      section.queryByRole("button", { name: "Prepare environment" }),
    ).not.toBeInTheDocument();
  });

  it("shows why preparing failed and reloads the task", async () => {
    const user = userEvent.setup();
    let loads = 0;
    core({
      list_tasks: () => {
        loads += 1;
        return loads === 1
          ? [linked]
          : [
              {
                ...linked,
                worktree: {
                  ...worktree,
                  status: "failed",
                  error_code: "checkout_failed",
                },
              },
            ];
      },
      list_repositories: () => [app],
      prepare_task_environment: () => {
        throw "environment_failed";
      },
    });
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Login/ }));
    await user.click(
      screen.getByRole("button", { name: "Prepare environment" }),
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "The environment could not be prepared.",
    );
    expect(
      await screen.findByText(/Git could not create the worktree/),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Discard" })).toBeInTheDocument();
  });

  it("does not offer preparing for closed tasks or without a base branch", async () => {
    const user = userEvent.setup();
    core({
      list_tasks: () => [
        { ...linked, status: "completed", completed_at: linked.created_at },
      ],
      list_repositories: () => [app],
    });
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Login/ }));

    expect(
      screen.getByText("Reopen the task to prepare an environment."),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Prepare environment" }),
    ).toBeDisabled();
  });

  it("discards a missing environment", async () => {
    const user = userEvent.setup();
    const discard = vi.fn(() => linked);
    core({
      list_tasks: () => [
        { ...linked, worktree: { ...worktree, status: "missing" } },
      ],
      list_repositories: () => [app],
      discard_task_environment: discard,
    });
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Login/ }));
    expect(
      screen.getByText(/no longer where Meelting created it/),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Discard" }));

    expect(discard).toHaveBeenCalledWith({ taskId: "t1" });
    expect(
      await screen.findByRole("button", { name: "Prepare environment" }),
    ).toBeInTheDocument();
  });
});

describe("removing an environment", () => {
  const ready = task("t1", "Login", {
    repository_id: "p1",
    base_ref: "main",
    worktree: {
      id: "w1",
      path: "/w/login",
      branch_name: "meelting/login-5c6d7e8f9a0b",
      base_ref: "main",
      base_commit_sha: "0123456789abcdef0123456789abcdef01234567",
      status: "ready",
      error_code: null,
      copies: [],
      trash_path: null,
      trashed_at: null,
      created_at: "2026-11-25T10:00:00.000Z",
      updated_at: "2026-11-25T10:00:00.000Z",
    },
  });

  it("shows a trashed environment without its actions", async () => {
    const user = userEvent.setup();
    core({
      list_tasks: () => [
        {
          ...ready,
          worktree: {
            ...ready.worktree!,
            trash_path: "/w/.trash/login",
            trashed_at: "2026-11-26T10:00:00.000Z",
          },
        },
      ],
      list_repositories: () => [app],
    });
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Login/ }));

    expect(screen.getByText("In the trash")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Remove environment" }),
    ).not.toBeInTheDocument();
  });

  it("lists the files that would be lost and keeps the environment", async () => {
    const user = userEvent.setup();
    const remove = vi.fn();
    core({
      list_tasks: () => [ready],
      list_repositories: () => [app],
      check_task_environment_removal: () => ({
        changes: [
          { kind: "ignored", path: ".env" },
          { kind: "tracked", path: "src/app.ts" },
        ],
        operation_in_progress: false,
        commits_ahead: 0,
      }),
      remove_task_environment: remove,
    });
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Login/ }));
    await user.click(
      screen.getByRole("button", { name: "Remove environment" }),
    );
    const dialog = within(
      await screen.findByRole("dialog", {
        name: "The environment cannot be removed yet",
      }),
    );

    expect(dialog.getByText(".env")).toBeInTheDocument();
    expect(dialog.getByText("Ignored")).toBeInTheDocument();
    expect(dialog.getByText("src/app.ts")).toBeInTheDocument();
    expect(
      dialog.queryByRole("button", { name: "Remove" }),
    ).not.toBeInTheDocument();
    await user.click(dialog.getByRole("button", { name: "Keep environment" }));
    expect(remove).not.toHaveBeenCalled();
  });

  it("confirms, warns that the branch keeps its commits and removes", async () => {
    const user = userEvent.setup();
    const remove = vi.fn(() => ({ ...ready, worktree: null }));
    core({
      list_tasks: () => [ready],
      list_repositories: () => [app],
      check_task_environment_removal: () => ({
        changes: [],
        operation_in_progress: false,
        commits_ahead: 2,
      }),
      remove_task_environment: remove,
    });
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Login/ }));
    await user.click(
      screen.getByRole("button", { name: "Remove environment" }),
    );
    const dialog = within(
      await screen.findByRole("dialog", { name: "Remove the environment?" }),
    );
    expect(
      dialog.getByText(
        "The branch meelting/login-5c6d7e8f9a0b is kept with its 2 commits that are not in the base branch.",
      ),
    ).toBeInTheDocument();
    await user.click(dialog.getByRole("button", { name: "Remove" }));

    expect(remove).toHaveBeenCalledWith({ taskId: "t1" });
    expect(
      await screen.findByRole("button", { name: "Prepare environment" }),
    ).toBeInTheDocument();
  });
});

describe("copying local files", () => {
  const ready = task("t1", "Login", {
    repository_id: "p1",
    base_ref: "main",
    worktree: {
      id: "w1",
      path: "/w/login",
      branch_name: "meelting/login-5c6d7e8f9a0b",
      base_ref: "main",
      base_commit_sha: "0123456789abcdef0123456789abcdef01234567",
      status: "ready",
      error_code: null,
      copies: [],
      trash_path: null,
      trashed_at: null,
      created_at: "2026-11-25T10:00:00.000Z",
      updated_at: "2026-11-25T10:00:00.000Z",
    },
  });
  const preview = {
    id: "s1",
    total_bytes: 2048,
    files: [
      { relative_path: ".env", size_bytes: 2048, secret: true, problem: null },
      {
        relative_path: "README.md",
        size_bytes: 10,
        secret: false,
        problem: "tracked",
      },
    ],
  };

  it("requires confirming secrets before copying and shows each result", async () => {
    const user = userEvent.setup();
    const copy = vi.fn(() => ({
      results: [{ relative_path: ".env", outcome: "copied", problem: null }],
      task: {
        ...ready,
        worktree: {
          ...ready.worktree!,
          copies: [
            {
              relative_path: ".env",
              size_bytes: 2048,
              copied_at: "2026-11-25T11:00:00.000Z",
            },
          ],
        },
      },
    }));
    core({
      list_tasks: () => [ready],
      list_repositories: () => [app],
      pick_task_environment_files: () => preview,
      copy_task_environment_files: copy,
    });
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Login/ }));
    await user.click(screen.getByRole("button", { name: "Copy local files" }));
    const dialog = within(
      await screen.findByRole("dialog", {
        name: "Copy local files into the environment",
      }),
    );
    expect(dialog.getByText("May hold secrets")).toBeInTheDocument();
    expect(
      dialog.getByText("Tracked by Git: it is already in the environment"),
    ).toBeInTheDocument();
    const submit = dialog.getByRole("button", { name: "Copy files" });
    expect(submit).toBeDisabled();

    await user.click(
      dialog.getByRole("checkbox", {
        name: "I understand and want to copy these files",
      }),
    );
    await user.click(submit);

    expect(copy).toHaveBeenCalledWith({
      taskId: "t1",
      selectionId: "s1",
      acceptSecrets: true,
    });
    expect(await dialog.findByText("Copied")).toBeInTheDocument();
    await user.click(dialog.getByRole("button", { name: "Done" }));
    expect(screen.getByText("Copied files")).toBeInTheDocument();
  });

  it("does nothing when the file picker is cancelled", async () => {
    const user = userEvent.setup();
    core({
      list_tasks: () => [ready],
      list_repositories: () => [app],
      pick_task_environment_files: () => null,
    });
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Login/ }));
    await user.click(screen.getByRole("button", { name: "Copy local files" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
