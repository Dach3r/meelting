import { invoke } from "@tauri-apps/api/core";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ReviewEntry } from "../../hooks/useEnvironmentReview";
import { I18nProvider } from "../../i18n/I18nProvider";
import { diff, entry, review, task, worktree } from "../../test/review";
import { TaskChanges } from "./TaskChanges";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

type Handlers = Record<string, (args: Record<string, unknown>) => unknown>;

function core(handlers: Handlers) {
  invokeMock.mockImplementation(async (command, args) => {
    const handler = handlers[command];
    if (!handler) throw new Error(`unexpected ${command}`);
    return handler((args ?? {}) as Record<string, unknown>);
  });
}

function renderChanges(options: Parameters<typeof task>) {
  return render(
    <I18nProvider locale="en">
      <TaskChanges task={task(...options)} />
    </I18nProvider>,
  );
}

const staged: Partial<ReviewEntry> = {
  staged: "modified",
  sources: ["staged"],
};
const unstaged: Partial<ReviewEntry> = {
  unstaged: "modified",
  sources: ["unstaged"],
};

beforeEach(() => {
  invokeMock.mockReset();
});

describe("header and states", () => {
  it("shows loading, then the branch, base, HEAD, commits ahead, drift and Git operation", async () => {
    core({
      review_task_environment: () => review([entry(0, "a.txt", staged)]),
    });
    renderChanges([]);

    expect(
      screen.getByText("Reading the changes of this environment…"),
    ).toBeVisible();
    expect(await screen.findByText("meelting/fix-login-abc")).toBeVisible();
    expect(screen.getByText(`main @ ${"a".repeat(12)}`)).toBeVisible();
    expect(screen.getByText("b".repeat(12))).toBeVisible();
    const operation = screen.getByText("Git operation").parentElement!;
    expect(within(operation).getByText("None")).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("review_task_environment", {
      taskId: "t1",
    });
    expect(screen.queryByText(/reviewed at/i)).toBeNull();
  });

  it("explains that nothing changed yet", async () => {
    core({ review_task_environment: () => review([]) });
    renderChanges([]);

    expect(await screen.findByText(/No changes yet/)).toBeVisible();
  });

  it("flags an operation in progress and a partial list", async () => {
    core({
      review_task_environment: () =>
        review([entry(0, "a.txt", staged)], {
          operation_in_progress: true,
          complete: false,
        }),
    });
    renderChanges([]);

    expect(
      await screen.findByText(/Git operation \(merge, rebase/),
    ).toBeVisible();
    expect(screen.getByText(/This list is incomplete/)).toBeVisible();
    expect(screen.getByText("In progress")).toBeVisible();
  });

  it.each([
    ["failed", null],
    ["missing", null],
    ["ready", "2026-11-25T10:00:00.000Z"],
  ] as const)(
    "does not read an environment that is %s or in the trash",
    (status, trashed) => {
      core({});
      renderChanges(["t1", worktree({ status, trashed_at: trashed })]);

      expect(screen.getByText(/environment is not ready/)).toBeVisible();
      expect(screen.queryByRole("button", { name: "Refresh" })).toBeNull();
      expect(invokeMock).not.toHaveBeenCalled();
    },
  );

  it.each([
    ["environment_identity_mismatch", /no longer match what Meelting created/],
    ["process_isolation_unavailable", /cannot guarantee that Git runs/],
    ["environment_filters_apply", /Git filters that run programs/],
    ["review_timeout", /took too long/],
  ])(
    "shows the backend error %s as is and lets the user retry",
    async (code, message) => {
      let calls = 0;
      core({
        review_task_environment: () => {
          calls += 1;
          if (calls === 1) throw code;
          return review([]);
        },
      });
      renderChanges([]);

      expect(await screen.findByRole("alert")).toHaveTextContent(message);
      expect(screen.queryByText("Base branch")).toBeNull();
      await userEvent.click(screen.getByRole("button", { name: "Retry" }));
      expect(await screen.findByText(/No changes yet/)).toBeVisible();
    },
  );
});

describe("drift", () => {
  it.each([
    ["moved", /has moved since this environment was prepared/],
    ["rewritten", /was rewritten/],
    ["unavailable", /not available/],
  ] as const)(
    "informs about %s without offering to sync",
    async (state, message) => {
      core({
        review_task_environment: () =>
          review([], {
            drift: { state, base_ref: "main", current_commit: null },
          }),
      });
      renderChanges([]);

      expect(await screen.findByText(message)).toBeVisible();
      expect(screen.queryByRole("alert")).toBeNull();
      expect(
        screen.queryByRole("button", {
          name: /merge|rebase|reset|sync|update/i,
        }),
      ).toBeNull();
    },
  );

  it("shows no notice when the base branch has not moved", async () => {
    core({ review_task_environment: () => review([]) });
    renderChanges([]);

    await screen.findByText(/No changes yet/);
    expect(screen.queryByRole("status", { name: "" })).toBeNull();
    expect(document.querySelector("[data-drift]")).toBeNull();
  });
});

describe("file groups", () => {
  const entries = [
    entry(0, "both.ts", {
      staged: "modified",
      unstaged: "modified",
      sources: ["staged", "unstaged"],
    }),
    entry(1, "new.ts", { staged: "added", sources: ["staged"] }),
    entry(2, "GUIDE.md", {
      staged: "renamed",
      original_path: "README.md",
      sources: ["staged"],
    }),
    entry(3, "clash.ts", { kind: "conflicted" }),
    entry(4, "notes.txt", { kind: "untracked" }),
    entry(5, "build/", { kind: "ignored", folder: true }),
    entry(6, "logo.png", {
      kind: "committed",
      sources: ["branch"],
      added: 0,
      deleted: 0,
      binary: true,
    }),
    entry(7, ".env.local", {
      staged: "added",
      sources: ["staged"],
      protected: true,
    }),
  ];

  it("keeps staged and unstaged apart and lists a file in both groups", async () => {
    core({ review_task_environment: () => review(entries) });
    renderChanges([]);

    const stagedGroup = await screen.findByRole("region", { name: /Staged/ });
    const unstagedGroup = screen.getByRole("region", { name: /Unstaged/ });
    expect(within(stagedGroup).getByText("both.ts")).toBeVisible();
    expect(within(unstagedGroup).getByText("both.ts")).toBeVisible();
    expect(within(unstagedGroup).queryByText("new.ts")).toBeNull();
    expect(within(stagedGroup).getByText("4")).toBeVisible();
    expect(within(unstagedGroup).getByText("1")).toBeVisible();
    expect(within(stagedGroup).getAllByText("Added")).toHaveLength(2);
    expect(within(stagedGroup).getByText("Renamed")).toBeVisible();
    expect(within(stagedGroup).getByText("from README.md")).toBeVisible();
  });

  it("shows conflicts, binary, protected, untracked and ignored with text labels", async () => {
    core({ review_task_environment: () => review(entries) });
    renderChanges([]);

    const conflicts = await screen.findByRole("region", { name: /Conflicts/ });
    expect(within(conflicts).getByText("Conflict")).toBeVisible();
    expect(within(conflicts).queryByRole("button")).toBeNull();
    expect(screen.getByText(/Conflicts are not shown as a diff/)).toBeVisible();
    expect(screen.getByText("Binary")).toBeVisible();
    expect(screen.getByText("Protected")).toBeVisible();
    expect(screen.getByRole("region", { name: /Untracked/ })).toHaveTextContent(
      "notes.txt",
    );
    expect(screen.getByRole("region", { name: /Ignored/ })).toHaveTextContent(
      "build/",
    );
    expect(screen.getByLabelText("0 added, 0 deleted lines")).toBeVisible();
  });
});

describe("diff", () => {
  const entries = [
    entry(0, "a.ts", { ...staged }),
    entry(1, "b.ts", { ...unstaged }),
    entry(2, ".env.local", { ...staged, protected: true }),
  ];

  it("reads a diff by opaque ids and renders numbered, labelled lines as text", async () => {
    core({
      review_task_environment: () => review(entries),
      read_task_environment_diff: () => diff(),
    });
    renderChanges([]);

    await userEvent.click(await screen.findByRole("button", { name: /a\.ts/ }));

    const table = await screen.findByRole("table", {
      name: /Changes of a\.ts/,
    });
    expect(invokeMock).toHaveBeenCalledWith("read_task_environment_diff", {
      taskId: "t1",
      request: { review_id: "review-1", entry_id: 0, source: "staged" },
    });
    expect(within(table).getByText("@@ -3,2 +3,3 @@")).toBeVisible();
    const rows = within(table).getAllByRole("row");
    expect(rows[2]).toHaveTextContent("4");
    expect(within(rows[2]).getByText("Removed line")).toBeInTheDocument();
    expect(within(rows[3]).getByText("Added line")).toBeInTheDocument();
    expect(within(rows[1]).getByText("Unchanged line")).toBeInTheDocument();
    expect(
      within(table).getByText("<img src=x onerror=alert(1)>"),
    ).toBeVisible();
    expect(table.querySelector("img")).toBeNull();
    expect(screen.getByRole("button", { name: /a\.ts/ })).toHaveAttribute(
      "aria-current",
      "true",
    );
  });

  it("numbers lines from the hunk header", async () => {
    core({
      review_task_environment: () => review(entries),
      read_task_environment_diff: () => diff(),
    });
    renderChanges([]);
    await userEvent.click(await screen.findByRole("button", { name: /a\.ts/ }));

    const rows = within(await screen.findByRole("table")).getAllByRole("row");
    const numbers = rows
      .slice(1)
      .map((row) =>
        Array.from(row.querySelectorAll("td[aria-hidden]")).map(
          (cell) => cell.textContent,
        ),
      );
    expect(numbers).toEqual([
      ["3", "3"],
      ["4", ""],
      ["", "4"],
      ["", "5"],
    ]);
  });

  it("explains protected files without asking the core for their contents", async () => {
    core({ review_task_environment: () => review(entries) });
    renderChanges([]);

    await userEvent.click(
      await screen.findByRole("button", { name: /\.env\.local/ }),
    );

    expect(await screen.findByText(/may hold secrets/)).toBeVisible();
    expect(invokeMock).not.toHaveBeenCalledWith(
      "read_task_environment_diff",
      expect.anything(),
    );
  });

  it("shows binary, no-change, partial and lossy results", async () => {
    const results = [
      diff({ binary: true, hunks: [] }),
      diff({ hunks: [] }),
      diff({ complete: false, lossy: true }),
    ];
    core({
      review_task_environment: () => review(entries),
      read_task_environment_diff: () => results.shift(),
    });
    renderChanges([]);

    await userEvent.click(await screen.findByRole("button", { name: /a\.ts/ }));
    expect(await screen.findByText(/binary file/)).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: /b\.ts/ }));
    expect(await screen.findByText(/no line changes/)).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: /a\.ts/ }));
    expect(await screen.findByText(/The diff is incomplete/)).toBeVisible();
    expect(screen.getByText(/replaced/)).toBeVisible();
  });

  it("shows a read error in the viewer and keeps the list", async () => {
    core({
      review_task_environment: () => review(entries),
      read_task_environment_diff: () => {
        throw "diff_too_large";
      },
    });
    renderChanges([]);

    await userEvent.click(await screen.findByRole("button", { name: /a\.ts/ }));

    expect(await screen.findByRole("alert")).toHaveTextContent(/too large/);
    expect(screen.getByRole("button", { name: /b\.ts/ })).toBeVisible();
  });

  it("discards the review when a read comes back stale and refreshes on demand", async () => {
    let reviews = 0;
    core({
      review_task_environment: () => {
        reviews += 1;
        return review([entry(reviews, "a.ts", staged)], {
          review_id: `review-${reviews}`,
        });
      },
      read_task_environment_diff: () => {
        throw "review_stale";
      },
    });
    renderChanges([]);

    await userEvent.click(await screen.findByRole("button", { name: /a\.ts/ }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      /changed since this review/,
    );
    expect(screen.queryByRole("button", { name: /a\.ts/ })).toBeNull();
    expect(screen.queryByRole("table")).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "Refresh" }));
    expect(await screen.findByRole("button", { name: /a\.ts/ })).toBeVisible();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("selects a file with the keyboard", async () => {
    core({
      review_task_environment: () => review(entries),
      read_task_environment_diff: () => diff(),
    });
    renderChanges([]);
    await screen.findByRole("button", { name: /a\.ts/ });

    await userEvent.tab();
    await userEvent.tab();
    expect(screen.getByRole("button", { name: /a\.ts/ })).toHaveFocus();
    await userEvent.keyboard("{Enter}");

    expect(await screen.findByRole("table")).toBeVisible();
  });

  it("clears the selection when the review is refreshed", async () => {
    core({
      review_task_environment: () => review(entries),
      read_task_environment_diff: () => diff(),
    });
    renderChanges([]);
    await userEvent.click(await screen.findByRole("button", { name: /a\.ts/ }));
    await screen.findByRole("table");

    await userEvent.click(screen.getByRole("button", { name: "Refresh" }));

    await waitFor(() => expect(screen.queryByRole("table")).toBeNull());
    expect(
      await screen.findByText("Select a file to read its changes."),
    ).toBeVisible();
  });
});

describe("untracked folders", () => {
  const folder = entry(0, "docs/", { kind: "untracked", folder: true });

  it("expands a folder through the core and lists its children", async () => {
    core({
      review_task_environment: () => review([folder]),
      list_task_environment_folder: () => ({
        entries: [
          entry(1, "docs/a.md", { kind: "untracked" }),
          entry(2, "docs/deep/", { kind: "untracked", folder: true }),
        ],
        complete: true,
      }),
    });
    renderChanges([]);

    const toggle = await screen.findByRole("button", {
      name: "Expand folder docs/",
    });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(toggle);

    expect(await screen.findByText("docs/a.md")).toBeVisible();
    expect(invokeMock).toHaveBeenCalledWith("list_task_environment_folder", {
      taskId: "t1",
      reviewId: "review-1",
      entryId: 0,
    });
    expect(
      screen.getByRole("button", { name: "Collapse folder docs/" }),
    ).toHaveAttribute("aria-expanded", "true");
    expect(
      screen.getByRole("button", { name: "Expand folder docs/deep/" }),
    ).toBeVisible();
  });

  it("keeps each folder's loading and error apart from the rest of the list", async () => {
    const other = entry(1, "src/", { kind: "untracked", folder: true });
    let release: (value: unknown) => void = () => {};
    core({
      review_task_environment: () => review([folder, other]),
      list_task_environment_folder: (args) =>
        args.entryId === 0
          ? new Promise((resolve) => {
              release = resolve;
            })
          : Promise.reject("review_timeout"),
    });
    renderChanges([]);

    await userEvent.click(
      await screen.findByRole("button", { name: "Expand folder docs/" }),
    );
    expect(screen.getByText("Reading the folder…")).toBeVisible();
    await userEvent.click(
      screen.getByRole("button", { name: "Expand folder src/" }),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(/took too long/);
    expect(screen.getByText("Reading the folder…")).toBeVisible();

    release({ entries: [], complete: false });
    expect(
      await screen.findByText("The folder has no files to show."),
    ).toBeVisible();
    expect(screen.getByText("The folder listing is incomplete.")).toBeVisible();
  });

  it("discards the review when a folder read is stale", async () => {
    core({
      review_task_environment: () => review([folder]),
      list_task_environment_folder: () => {
        throw "review_stale";
      },
    });
    renderChanges([]);

    await userEvent.click(
      await screen.findByRole("button", { name: "Expand folder docs/" }),
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      /changed since this review/,
    );
    expect(screen.queryByText("docs/")).toBeNull();
  });
});
