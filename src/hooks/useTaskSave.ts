import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import type { Task, TaskDraft, TaskOrigin } from "./useTasks";

export function useTaskSave() {
  const [busy, setBusy] = useState(false);
  const [errorKey, setErrorKey] = useState<string | null>(null);

  async function save(action: () => Promise<Task>) {
    setBusy(true);
    setErrorKey(null);
    try {
      return await action();
    } catch (error) {
      setErrorKey(String(error));
      return null;
    } finally {
      setBusy(false);
    }
  }

  const createTask = (draft: TaskDraft, origin: TaskOrigin | null) =>
    save(() => invoke<Task>("create_task", { draft, origin }));

  const updateTask = (taskId: string, draft: TaskDraft) =>
    save(() => invoke<Task>("update_task", { taskId, draft }));

  return { busy, errorKey, createTask, updateTask };
}
