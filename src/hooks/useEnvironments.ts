import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import type { Worktree } from "./useTasks";

export type Environment = {
  task_id: string;
  task_title: string;
  repository_name: string;
  worktree: Worktree;
};

export type EnvironmentFailure = { task_id: string; error: string };

type BatchCommand =
  | "trash_task_environments"
  | "restore_task_environments"
  | "purge_task_environments";

export function useEnvironments() {
  const [environments, setEnvironments] = useState<Environment[] | null>(null);
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [failures, setFailures] = useState<EnvironmentFailure[]>([]);
  const [busy, setBusy] = useState(false);

  const reload = useCallback(async () => {
    try {
      setEnvironments(await invoke<Environment[]>("list_task_environments"));
      setErrorKey(null);
    } catch (error) {
      setErrorKey(String(error));
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  async function batch(command: BatchCommand, taskIds: string[]) {
    setBusy(true);
    setFailures([]);
    try {
      setFailures(await invoke<EnvironmentFailure[]>(command, { taskIds }));
    } catch (error) {
      setErrorKey(String(error));
    } finally {
      await reload();
      setBusy(false);
    }
  }

  return {
    environments,
    errorKey,
    failures,
    busy,
    trash: (taskIds: string[]) => batch("trash_task_environments", taskIds),
    restore: (taskIds: string[]) => batch("restore_task_environments", taskIds),
    purge: (taskIds: string[]) => batch("purge_task_environments", taskIds),
  };
}
