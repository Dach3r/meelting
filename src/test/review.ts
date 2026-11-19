import type {
  EnvironmentReview,
  FileDiff,
  ReviewEntry,
} from "../hooks/useEnvironmentReview";
import type { Task, Worktree } from "../hooks/useTasks";

export function entry(
  id: number,
  path: string,
  overrides: Partial<ReviewEntry> = {},
): ReviewEntry {
  return {
    id,
    kind: "changed",
    path,
    original_path: null,
    lossy_path: false,
    staged: null,
    unstaged: null,
    sources: [],
    protected: false,
    folder: false,
    added: null,
    deleted: null,
    binary: false,
    ...overrides,
  };
}

export function review(
  entries: ReviewEntry[],
  overrides: Partial<EnvironmentReview> = {},
): EnvironmentReview {
  return {
    review_id: "review-1",
    branch: "meelting/fix-login-abc",
    base_commit_sha: "a".repeat(40),
    head_commit_sha: "b".repeat(40),
    commits_ahead: 2,
    drift: { state: "up_to_date", base_ref: "main", current_commit: "a".repeat(40) },
    operation_in_progress: false,
    entries,
    complete: true,
    ...overrides,
  };
}

export function diff(overrides: Partial<FileDiff> = {}): FileDiff {
  return {
    binary: false,
    hunks: [
      {
        header: "@@ -3,2 +3,3 @@",
        lines: [
          { kind: "context", text: "keep" },
          { kind: "removed", text: "old" },
          { kind: "added", text: "new" },
          { kind: "added", text: "<img src=x onerror=alert(1)>" },
        ],
      },
    ],
    complete: true,
    lossy: false,
    ...overrides,
  };
}

export function worktree(overrides: Partial<Worktree> = {}): Worktree {
  return {
    id: "w1",
    path: "/work/app",
    branch_name: "meelting/fix-login-abc",
    base_ref: "main",
    base_commit_sha: "a".repeat(40),
    status: "ready",
    error_code: null,
    copies: [],
    trash_path: null,
    trashed_at: null,
    created_at: "2026-11-24T10:00:00.000Z",
    updated_at: "2026-11-24T10:00:00.000Z",
    ...overrides,
  };
}

export function task(id = "t1", wt: Worktree | null = worktree()): Task {
  return {
    id,
    title: "Fix login",
    description: "",
    status: "in_progress",
    priority: "medium",
    repository_id: "r1",
    base_ref: "main",
    origin_room_id: null,
    origin_room_name: null,
    origin_message_id: null,
    linked_rooms: [],
    agent_ids: [],
    worktree: wt,
    completed_at: null,
    created_at: "2026-11-24T10:00:00.000Z",
    updated_at: "2026-11-24T10:00:00.000Z",
  };
}
