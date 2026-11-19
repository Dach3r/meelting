import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

export type TaskStatus =
  | "pending"
  | "in_progress"
  | "blocked"
  | "completed"
  | "canceled";

export type TaskPriority = "low" | "medium" | "high";

export type Task = {
  id: string;
  title: string;
  description: string;
  status: TaskStatus;
  priority: TaskPriority;
  repository_id: string | null;
  base_ref: string | null;
  origin_room_id: string | null;
  origin_room_name: string | null;
  origin_message_id: string | null;
  agent_ids: string[];
  completed_at: string | null;
  created_at: string;
  updated_at: string;
};

export type TaskDraft = {
  title: string;
  description: string;
  priority: TaskPriority;
  repository_id: string | null;
  base_ref: string | null;
  agent_ids: string[];
};

export type TaskOrigin = { room_id: string; message_id: string | null };

export const taskStatuses: TaskStatus[] = [
  "pending",
  "in_progress",
  "blocked",
  "completed",
  "canceled",
];

export const nextStatuses: Record<TaskStatus, TaskStatus[]> = {
  pending: ["in_progress", "blocked", "completed", "canceled"],
  in_progress: ["pending", "blocked", "completed", "canceled"],
  blocked: ["pending", "in_progress", "canceled"],
  completed: ["in_progress"],
  canceled: ["pending"],
};

export function useTasks() {
  const [tasks, setTasks] = useState<Task[] | null>(null);
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [actionErrorKey, setActionErrorKey] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const reload = useCallback(async () => {
    try {
      setTasks(await invoke<Task[]>("list_tasks"));
      setErrorKey(null);
    } catch (error) {
      setErrorKey(String(error));
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  function replace(task: Task) {
    setTasks((current) =>
      current
        ? [task, ...current.filter((other) => other.id !== task.id)]
        : [task],
    );
  }

  async function changeStatus(taskId: string, status: TaskStatus) {
    setBusy(true);
    setActionErrorKey(null);
    try {
      replace(await invoke<Task>("change_task_status", { taskId, status }));
    } catch (error) {
      setActionErrorKey(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function deleteTask(taskId: string) {
    setBusy(true);
    setActionErrorKey(null);
    try {
      setTasks(await invoke<Task[]>("delete_task", { taskId }));
      return true;
    } catch (error) {
      setActionErrorKey(String(error));
      return false;
    } finally {
      setBusy(false);
    }
  }

  return {
    tasks,
    errorKey,
    actionErrorKey,
    busy,
    replace,
    changeStatus,
    deleteTask,
    clearActionError: () => setActionErrorKey(null),
  };
}
