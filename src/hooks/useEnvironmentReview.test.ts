import { invoke } from "@tauri-apps/api/core";
import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { diff, entry, review } from "../test/review";
import {
  groupEntries,
  useEnvironmentReview,
  type EnvironmentReview,
  type FileDiff,
  type ReviewEntry,
} from "./useEnvironmentReview";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

const staged: Partial<ReviewEntry> = {
  staged: "modified",
  sources: ["staged"],
};

beforeEach(() => {
  invokeMock.mockReset();
});

describe("groupEntries", () => {
  it("puts a file that is staged and unstaged in both groups", () => {
    const groups = groupEntries([
      entry(0, "a", {
        staged: "modified",
        unstaged: "modified",
        sources: ["staged", "unstaged"],
      }),
      entry(1, "b", { kind: "conflicted" }),
      entry(2, "c", { kind: "committed", sources: ["branch"] }),
    ]);

    expect(groups.staged.map((row) => row.entry.path)).toEqual(["a"]);
    expect(groups.unstaged.map((row) => row.entry.path)).toEqual(["a"]);
    expect(groups.conflicted[0].source).toBeNull();
    expect(groups.committed[0].source).toBe("branch");
  });
});

describe("useEnvironmentReview", () => {
  it("ignores a late review from a previous refresh", async () => {
    const first = deferred<EnvironmentReview>();
    const second = deferred<EnvironmentReview>();
    invokeMock
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise);
    const { result } = renderHook(() => useEnvironmentReview("t1", true));

    act(() => result.current.refresh());
    second.resolve(review([entry(0, "fresh.ts", staged)]));
    await waitFor(() => expect(result.current.phase).toBe("ready"));
    first.resolve(review([entry(0, "old.ts", staged)]));
    await act(async () => {});

    expect(result.current.review?.entries[0].path).toBe("fresh.ts");
  });

  it("drops a late response from the previous task", async () => {
    const slow = deferred<EnvironmentReview>();
    invokeMock.mockImplementation((_command, args) =>
      (args as { taskId: string }).taskId === "t1"
        ? slow.promise
        : Promise.resolve(review([entry(0, "second-task.ts", staged)])),
    );
    const { result, rerender } = renderHook(
      ({ taskId }) => useEnvironmentReview(taskId, true),
      { initialProps: { taskId: "t1" } },
    );

    rerender({ taskId: "t2" });
    await waitFor(() => expect(result.current.phase).toBe("ready"));
    slow.resolve(review([entry(0, "first-task.ts", staged)]));
    await act(async () => {});

    expect(result.current.review?.entries[0].path).toBe("second-task.ts");
  });

  it("keeps the newest diff when answers arrive out of order", async () => {
    const slow = deferred<FileDiff>();
    const fast = deferred<FileDiff>();
    invokeMock.mockResolvedValueOnce(
      review([entry(0, "a", staged), entry(1, "b", staged)]),
    );
    const { result } = renderHook(() => useEnvironmentReview("t1", true));
    await waitFor(() => expect(result.current.phase).toBe("ready"));
    invokeMock
      .mockReturnValueOnce(slow.promise)
      .mockReturnValueOnce(fast.promise);

    act(() => result.current.selectRow(result.current.groups!.staged[0]));
    act(() => result.current.selectRow(result.current.groups!.staged[1]));
    fast.resolve(diff({ hunks: [] }));
    await waitFor(() => expect(result.current.diff?.phase).toBe("ready"));
    slow.resolve(diff());
    await act(async () => {});

    expect(result.current.selected?.entry.path).toBe("b");
    expect(
      result.current.diff?.phase === "ready" && result.current.diff.diff.hunks,
    ).toEqual([]);
  });

  it("drops a diff that arrives after a refresh", async () => {
    const late = deferred<FileDiff>();
    invokeMock.mockImplementation((command) =>
      command === "review_task_environment"
        ? Promise.resolve(review([entry(0, "a", staged)]))
        : late.promise,
    );
    const { result } = renderHook(() => useEnvironmentReview("t1", true));
    await waitFor(() => expect(result.current.phase).toBe("ready"));

    act(() => result.current.selectRow(result.current.groups!.staged[0]));
    act(() => result.current.refresh());
    late.resolve(diff());
    await waitFor(() => expect(result.current.phase).toBe("ready"));

    expect(result.current.diff).toBeNull();
    expect(result.current.selected).toBeNull();
  });

  it("does not update state after unmounting", async () => {
    const pending = deferred<EnvironmentReview>();
    invokeMock.mockReturnValue(pending.promise);
    const error = vi.spyOn(console, "error").mockImplementation(() => {});
    const { result, unmount } = renderHook(() =>
      useEnvironmentReview("t1", true),
    );

    unmount();
    pending.resolve(review([]));
    await act(async () => {});

    expect(result.current.review).toBeNull();
    expect(error).not.toHaveBeenCalled();
    error.mockRestore();
  });

  it("does not read anything while disabled", () => {
    renderHook(() => useEnvironmentReview("t1", false));

    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("requests a folder again only after an error", async () => {
    const folder = entry(0, "docs/", { kind: "untracked", folder: true });
    invokeMock.mockImplementation((command) =>
      command === "review_task_environment"
        ? Promise.resolve(review([folder]))
        : Promise.reject("review_timeout"),
    );
    const { result } = renderHook(() => useEnvironmentReview("t1", true));
    await waitFor(() => expect(result.current.phase).toBe("ready"));

    act(() => result.current.toggleFolder(folder));
    await waitFor(() => expect(result.current.folders[0]?.phase).toBe("error"));
    act(() => result.current.toggleFolder(folder));
    act(() => result.current.toggleFolder(folder));
    await waitFor(() => expect(result.current.folders[0]?.phase).toBe("error"));

    const folderCalls = invokeMock.mock.calls.filter(
      ([command]) => command === "list_task_environment_folder",
    );
    expect(folderCalls).toHaveLength(2);
  });
});
