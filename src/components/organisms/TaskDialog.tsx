import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { translateError } from "../../i18n/translate";
import { useRepositories } from "../../hooks/useRepositories";
import { useTaskSave } from "../../hooks/useTaskSave";
import type { Task, TaskDraft, TaskOrigin } from "../../hooks/useTasks";
import type { Agent } from "../../hooks/useTeams";
import { Alert } from "../molecules/Alert";
import { Modal } from "../molecules/Modal";
import { SegmentedControl } from "../molecules/SegmentedControl";
import { TaskForm } from "../molecules/TaskForm";
import { TaskLinkPanel } from "./TaskLinkPanel";

type Props = {
  agents: Agent[];
  task?: Task;
  origin?: TaskOrigin;
  description?: string;
  onSaved: (task: Task) => void;
  onClose: () => void;
};

export function TaskDialog({
  agents,
  task,
  origin,
  description = "",
  onSaved,
  onClose,
}: Props) {
  const { t, locale } = useI18n();
  const { repositories, errorKey, loadBranches } = useRepositories();
  const {
    busy,
    errorKey: saveErrorKey,
    createTask,
    updateTask,
    linkTask,
  } = useTaskSave();
  const [mode, setMode] = useState<"create" | "link">("create");
  const initial: TaskDraft = task
    ? {
        description: task.description,
        priority: task.priority,
        repository_id: task.repository_id,
        base_ref: task.base_ref,
        agent_ids: task.agent_ids,
      }
    : {
        description,
        priority: "medium",
        repository_id: null,
        base_ref: null,
        agent_ids: [],
      };

  async function submit(draft: TaskDraft) {
    const saved = task
      ? await updateTask(task.id, draft)
      : await createTask(draft, origin ?? null);
    if (saved) onSaved(saved);
  }

  async function link(existing: Task) {
    const saved = await linkTask(existing.id, origin!);
    if (saved) onSaved(saved);
  }

  return (
    <Modal
      title={task ? t("tasks.form.titleEdit") : t("tasks.form.titleNew")}
      onClose={onClose}
    >
      {errorKey && (
        <div className="px-6 pt-4 sm:px-10">
          <Alert message={translateError(locale, errorKey)} />
        </div>
      )}
      {origin && !task && (
        <div className="px-6 pt-4 sm:px-10">
          <SegmentedControl
            label={t("tasks.link.mode")}
            value={mode}
            options={[
              { value: "create", label: t("tasks.link.create") },
              { value: "link", label: t("tasks.link.link") },
            ]}
            onChange={setMode}
          />
        </div>
      )}
      {origin && mode === "link" && (
        <TaskLinkPanel
          origin={origin}
          busy={busy}
          errorMessage={
            saveErrorKey ? translateError(locale, saveErrorKey) : null
          }
          onLink={link}
        />
      )}
      {repositories && mode === "create" && (
        <TaskForm
          agents={agents}
          repositories={repositories}
          initial={initial}
          editing={task !== undefined}
          busy={busy}
          errorMessage={
            saveErrorKey ? translateError(locale, saveErrorKey) : null
          }
          onLoadBranches={loadBranches}
          onSubmit={submit}
          onCancel={onClose}
        />
      )}
    </Modal>
  );
}
