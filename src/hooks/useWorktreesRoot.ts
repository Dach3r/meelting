import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export type StorageIssue = "network" | "removable" | "synced" | "unknown";

export type WorktreesRoot = {
  path: string;
  custom: boolean;
  issue: StorageIssue | null;
};

export function useWorktreesRoot() {
  const [root, setRoot] = useState<WorktreesRoot | null>(null);
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let current = true;
    invoke<WorktreesRoot>("get_worktrees_root")
      .then((loaded) => {
        if (current) setRoot(loaded);
      })
      .catch((error) => {
        if (current) setErrorKey(String(error));
      });
    return () => {
      current = false;
    };
  }, []);

  async function changeRoot() {
    setBusy(true);
    setErrorKey(null);
    try {
      const changed = await invoke<WorktreesRoot | null>(
        "change_worktrees_root",
      );
      if (changed) setRoot(changed);
    } catch (error) {
      setErrorKey(String(error));
    } finally {
      setBusy(false);
    }
  }

  return { root, errorKey, busy, changeRoot };
}
