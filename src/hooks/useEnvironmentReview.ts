import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

export type ChangeCode =
  | "added"
  | "modified"
  | "deleted"
  | "renamed"
  | "copied"
  | "type_changed";

export type EntryKind =
  | "committed"
  | "changed"
  | "conflicted"
  | "untracked"
  | "ignored";

export type DiffSource = "staged" | "unstaged" | "branch";

export type DriftState = "up_to_date" | "moved" | "rewritten" | "unavailable";

export type ReviewEntry = {
  id: number;
  kind: EntryKind;
  path: string;
  original_path: string | null;
  lossy_path: boolean;
  staged: ChangeCode | null;
  unstaged: ChangeCode | null;
  sources: DiffSource[];
  protected: boolean;
  folder: boolean;
  added: number | null;
  deleted: number | null;
  binary: boolean;
};

export type EnvironmentReview = {
  review_id: string;
  branch: string;
  base_commit_sha: string;
  head_commit_sha: string | null;
  commits_ahead: number;
  drift: {
    state: DriftState;
    base_ref: string;
    current_commit: string | null;
  };
  operation_in_progress: boolean;
  entries: ReviewEntry[];
  complete: boolean;
};

export type DiffLineKind = "context" | "added" | "removed" | "no_newline";

export type FileDiff = {
  binary: boolean;
  hunks: { header: string; lines: { kind: DiffLineKind; text: string }[] }[];
  complete: boolean;
  lossy: boolean;
};

type FolderListing = { entries: ReviewEntry[]; complete: boolean };

export type GroupKey =
  | "conflicted"
  | "staged"
  | "unstaged"
  | "untracked"
  | "committed"
  | "ignored";

export const groupOrder: GroupKey[] = [
  "conflicted",
  "staged",
  "unstaged",
  "untracked",
  "committed",
  "ignored",
];

export type Row = { entry: ReviewEntry; source: DiffSource | null };

export function groupEntries(entries: ReviewEntry[]): Record<GroupKey, Row[]> {
  const groups: Record<GroupKey, Row[]> = {
    conflicted: [],
    staged: [],
    unstaged: [],
    untracked: [],
    committed: [],
    ignored: [],
  };
  for (const entry of entries) {
    if (entry.kind === "changed") {
      if (entry.staged) groups.staged.push({ entry, source: "staged" });
      if (entry.unstaged) groups.unstaged.push({ entry, source: "unstaged" });
    } else if (entry.kind === "committed") {
      groups.committed.push({ entry, source: "branch" });
    } else {
      groups[entry.kind].push({ entry, source: null });
    }
  }
  return groups;
}

export function rowKey(row: Row) {
  return `${row.entry.id}:${row.source ?? "none"}`;
}

export type Phase = "loading" | "ready" | "error" | "stale";

export type DiffState =
  | { phase: "loading" }
  | { phase: "ready"; diff: FileDiff }
  | { phase: "protected" }
  | { phase: "error"; errorKey: string };

export type FolderState =
  | { phase: "loading" }
  | { phase: "ready"; entries: ReviewEntry[]; complete: boolean }
  | { phase: "error"; errorKey: string };

export function useEnvironmentReview(taskId: string, enabled: boolean) {
  const [phase, setPhase] = useState<Phase>("loading");
  const [review, setReview] = useState<EnvironmentReview | null>(null);
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [selected, setSelected] = useState<Row | null>(null);
  const [diff, setDiff] = useState<DiffState | null>(null);
  const [folders, setFolders] = useState<Record<number, FolderState>>({});
  const [expanded, setExpanded] = useState<number[]>([]);
  const session = useRef(0);
  const diffRequest = useRef(0);

  const clear = useCallback(() => {
    setReview(null);
    setSelected(null);
    setDiff(null);
    setFolders({});
    setExpanded([]);
  }, []);

  const markStale = useCallback(() => {
    session.current += 1;
    clear();
    setErrorKey("review_stale");
    setPhase("stale");
  }, [clear]);

  const load = useCallback(() => {
    const id = ++session.current;
    diffRequest.current += 1;
    clear();
    setErrorKey(null);
    setPhase("loading");
    invoke<EnvironmentReview>("review_task_environment", { taskId }).then(
      (loaded) => {
        if (session.current !== id) return;
        setReview(loaded);
        setPhase("ready");
      },
      (error) => {
        if (session.current !== id) return;
        setErrorKey(String(error));
        setPhase("error");
      },
    );
  }, [taskId, clear]);

  useEffect(() => {
    if (enabled) load();
    return () => {
      session.current += 1;
      diffRequest.current += 1;
    };
  }, [enabled, load]);

  function selectRow(row: Row) {
    if (!review || !row.source) return;
    const id = session.current;
    const request = ++diffRequest.current;
    setSelected(row);
    if (row.entry.protected) {
      setDiff({ phase: "protected" });
      return;
    }
    setDiff({ phase: "loading" });
    invoke<FileDiff>("read_task_environment_diff", {
      taskId,
      request: {
        review_id: review.review_id,
        entry_id: row.entry.id,
        source: row.source,
      },
    }).then(
      (loaded) => {
        if (session.current !== id || diffRequest.current !== request) return;
        setDiff({ phase: "ready", diff: loaded });
      },
      (error) => {
        if (session.current !== id || diffRequest.current !== request) return;
        if (String(error) === "review_stale") markStale();
        else setDiff({ phase: "error", errorKey: String(error) });
      },
    );
  }

  function toggleFolder(entry: ReviewEntry) {
    if (expanded.includes(entry.id)) {
      setExpanded((current) => current.filter((open) => open !== entry.id));
      return;
    }
    setExpanded((current) => [...current, entry.id]);
    if (!review || folders[entry.id]?.phase) {
      if (folders[entry.id]?.phase !== "error") return;
    }
    const id = session.current;
    setFolders((current) => ({ ...current, [entry.id]: { phase: "loading" } }));
    invoke<FolderListing>("list_task_environment_folder", {
      taskId,
      reviewId: review.review_id,
      entryId: entry.id,
    }).then(
      (listing) => {
        if (session.current !== id) return;
        setFolders((current) => ({
          ...current,
          [entry.id]: { phase: "ready", ...listing },
        }));
      },
      (error) => {
        if (session.current !== id) return;
        if (String(error) === "review_stale") markStale();
        else {
          setFolders((current) => ({
            ...current,
            [entry.id]: { phase: "error", errorKey: String(error) },
          }));
        }
      },
    );
  }

  const groups = useMemo(
    () => (review ? groupEntries(review.entries) : null),
    [review],
  );

  return {
    phase,
    review,
    groups,
    errorKey,
    selected,
    diff,
    folders,
    expanded,
    refresh: load,
    selectRow,
    toggleFolder,
  };
}
