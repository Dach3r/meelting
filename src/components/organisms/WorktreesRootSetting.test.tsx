import { invoke } from "@tauri-apps/api/core";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { I18nProvider } from "../../i18n/I18nProvider";
import { WorktreesRootSetting } from "./WorktreesRootSetting";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

function core(handlers: Record<string, () => unknown>) {
  invokeMock.mockImplementation(async (command) => {
    const handler = handlers[command];
    if (!handler) throw new Error(`unexpected ${command}`);
    return handler();
  });
}

function renderSetting() {
  render(
    <I18nProvider locale="en">
      <WorktreesRootSetting />
    </I18nProvider>,
  );
}

const defaultRoot = {
  path: "/Users/ana/Meelting/worktrees",
  custom: false,
  issue: null,
};

beforeEach(() => {
  invokeMock.mockReset();
});

describe("worktree folder", () => {
  it("shows the default folder and that it is fixed local storage", async () => {
    core({ get_worktrees_root: () => defaultRoot });
    renderSetting();

    expect(
      await screen.findByText("/Users/ana/Meelting/worktrees"),
    ).toBeInTheDocument();
    expect(screen.getByText("Default folder")).toBeInTheDocument();
    expect(screen.getByText("Fixed local storage.")).toBeInTheDocument();
  });

  it("explains why the current folder cannot be used", async () => {
    core({
      get_worktrees_root: () => ({ ...defaultRoot, issue: "synced" }),
    });
    renderSetting();

    expect(
      await screen.findByText(/synchronized by a cloud service/),
    ).toBeInTheDocument();
  });

  it("changes the folder with the native picker", async () => {
    const user = userEvent.setup();
    core({
      get_worktrees_root: () => defaultRoot,
      change_worktrees_root: () => ({
        path: "/Volumes/Work/worktrees",
        custom: true,
        issue: null,
      }),
    });
    renderSetting();

    await user.click(
      await screen.findByRole("button", { name: "Change folder" }),
    );

    expect(
      await screen.findByText("/Volumes/Work/worktrees"),
    ).toBeInTheDocument();
    expect(screen.getByText("Chosen folder")).toBeInTheDocument();
  });

  it("keeps the folder when the picker is cancelled or the folder is rejected", async () => {
    const user = userEvent.setup();
    let calls = 0;
    core({
      get_worktrees_root: () => defaultRoot,
      change_worktrees_root: () => {
        calls += 1;
        if (calls === 1) return null;
        throw "storage_network";
      },
    });
    renderSetting();
    const change = await screen.findByRole("button", { name: "Change folder" });

    await user.click(change);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    await user.click(change);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "That folder is on a network drive. Use fixed local storage.",
    );
    expect(
      screen.getByText("/Users/ana/Meelting/worktrees"),
    ).toBeInTheDocument();
  });
});
