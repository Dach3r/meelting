import { invoke } from "@tauri-apps/api/core";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Environment } from "../../hooks/useEnvironments";
import type { Worktree } from "../../hooks/useTasks";
import { I18nProvider } from "../../i18n/I18nProvider";
import { EnvironmentsPage } from "./EnvironmentsPage";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

function environment(
  taskId: string,
  title: string,
  overrides: Partial<Worktree> = {},
): Environment {
  return {
    task_id: taskId,
    task_title: title,
    repository_name: "my-app",
    worktree: {
      id: `w-${taskId}`,
      path: `/w/${taskId}`,
      branch_name: `meelting/${taskId}-5c6d7e8f9a0b`,
      base_ref: "main",
      base_commit_sha: "0123456789abcdef0123456789abcdef01234567",
      status: "ready",
      error_code: null,
      copies: [],
      trash_path: null,
      trashed_at: null,
      created_at: "2026-11-24T10:00:00.000Z",
      updated_at: "2026-11-24T10:00:00.000Z",
      ...overrides,
    },
  };
}

function trashedEnvironment(taskId: string, title: string, daysAgo: number) {
  return environment(taskId, title, {
    trash_path: `/w/.trash/${taskId}`,
    trashed_at: new Date(Date.now() - daysAgo * 86_400_000).toISOString(),
  });
}

function core(initial: Environment[], failures: unknown[] = []) {
  let list = initial;
  const calls: { command: string; taskIds: unknown }[] = [];
  invokeMock.mockImplementation(async (command, args) => {
    if (command === "list_task_environments") return list;
    const taskIds = (args as { taskIds: string[] }).taskIds;
    calls.push({ command, taskIds });
    list = list.filter((item) => !taskIds.includes(item.task_id));
    return failures;
  });
  return calls;
}

function renderPage(mode: "files" | "trash") {
  return render(
    <I18nProvider locale="en">
      <EnvironmentsPage mode={mode} />
    </I18nProvider>,
  );
}

beforeEach(() => {
  invokeMock.mockReset();
});

describe("files", () => {
  it("lists environments outside the trash and moves the selected ones after confirming", async () => {
    const calls = core([
      environment("t1", "Login"),
      environment("t2", "Payments"),
      trashedEnvironment("t3", "Old", 2),
    ]);
    const user = userEvent.setup();
    renderPage("files");

    const list = await screen.findByRole("list", { name: "Files" });
    expect(within(list).getAllByRole("listitem")).toHaveLength(2);
    expect(screen.queryByText("Old")).toBeNull();
    expect(screen.getByText("/w/t1")).toBeTruthy();

    await user.click(screen.getByRole("checkbox", { name: "Login" }));
    await user.click(screen.getByRole("button", { name: "Move to trash (1)" }));
    const dialog = screen.getByRole("dialog", { name: "Move 1 to the trash?" });
    await user.click(
      within(dialog).getByRole("button", { name: "Move to trash" }),
    );

    await waitFor(() =>
      expect(calls).toEqual([
        { command: "trash_task_environments", taskIds: ["t1"] },
      ]),
    );
    await waitFor(() => expect(screen.queryByText("Login")).toBeNull());
    expect(screen.getByText("Payments")).toBeTruthy();
  });

  it("empties every ready environment at once and skips the ones that are not ready", async () => {
    const calls = core([
      environment("t1", "Login"),
      environment("t2", "Broken", {
        status: "failed",
        error_code: "checkout_failed",
      }),
      environment("t3", "Payments"),
    ]);
    const user = userEvent.setup();
    renderPage("files");

    await screen.findByText("Login");
    expect(
      (screen.getByRole("checkbox", { name: "Broken" }) as HTMLInputElement)
        .disabled,
    ).toBe(true);
    await user.click(screen.getByRole("button", { name: "Empty all" }));
    await user.click(
      within(
        screen.getByRole("dialog", { name: "Move 2 to the trash?" }),
      ).getByRole("button", { name: "Move to trash" }),
    );

    await waitFor(() =>
      expect(calls).toEqual([
        { command: "trash_task_environments", taskIds: ["t1", "t3"] },
      ]),
    );
  });

  it("explains each environment that could not be moved", async () => {
    core(
      [environment("t1", "Login")],
      [{ task_id: "t1", error: "environment_identity_mismatch" }],
    );
    const user = userEvent.setup();
    renderPage("files");

    await user.click(
      await screen.findByRole("checkbox", { name: "Select all" }),
    );
    await user.click(screen.getByRole("button", { name: "Move to trash (1)" }));
    await user.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: "Move to trash",
      }),
    );

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("1 could not be processed");
    expect(alert.textContent).toContain("Login");
  });
});

describe("trash", () => {
  it("shows the days left and restores without asking", async () => {
    const calls = core([
      trashedEnvironment("t1", "Login", 5),
      environment("t2", "Active"),
    ]);
    const user = userEvent.setup();
    renderPage("trash");

    expect(await screen.findByText("Deleted in 25 days")).toBeTruthy();
    expect(screen.getByText("/w/.trash/t1")).toBeTruthy();
    expect(screen.queryByText("Active")).toBeNull();

    await user.click(screen.getByRole("checkbox", { name: "Login" }));
    await user.click(screen.getByRole("button", { name: "Restore (1)" }));

    await waitFor(() =>
      expect(calls).toEqual([
        { command: "restore_task_environments", taskIds: ["t1"] },
      ]),
    );
    expect(await screen.findByText("The trash is empty.")).toBeTruthy();
  });

  it("deletes permanently only after confirming and can empty the whole trash", async () => {
    const calls = core([
      trashedEnvironment("t1", "Login", 1),
      trashedEnvironment("t2", "Payments", 29),
    ]);
    const user = userEvent.setup();
    renderPage("trash");

    await screen.findByText("Login");
    await user.click(screen.getByRole("button", { name: "Empty trash" }));
    const dialog = screen.getByRole("dialog", {
      name: "Delete 2 permanently?",
    });
    expect(dialog.textContent).toContain("the branch with all its commits");
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(calls).toEqual([]);

    await user.click(screen.getByRole("button", { name: "Empty trash" }));
    await user.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: "Delete permanently",
      }),
    );

    await waitFor(() =>
      expect(calls).toEqual([
        { command: "purge_task_environments", taskIds: ["t1", "t2"] },
      ]),
    );
  });
});
