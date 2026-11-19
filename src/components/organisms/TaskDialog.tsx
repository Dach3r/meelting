import { useI18n } from "../../i18n/I18nProvider";
import { translateError } from "../../i18n/translate";
import { useRepositories } from "../../hooks/useRepositories";
import { useTaskSave } from "../../hooks/useTaskSave";
import type { Task, TaskDraft, TaskOrigin } from "../../hooks/useTasks";
import type { Agent } from "../../hooks/useTeams";
import { Alert } from "../molecules/Alert";
import { Modal } from "../molecules/Modal";
import { TaskForm } from "../molecules/TaskForm";

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
  const { busy, errorKey: saveErrorKey, createTask, updateTask } =
    useTaskSave();
  const initial: TaskDraft = task
    ? {
        title: task.title,
        description: task.description,
        priority: task.priority,
        repository_id: task.repository_id,
        base_ref: task.base_ref,
        agent_ids: task.agent_ids,
      }
    : {
        title: "",
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
      {repositories && (
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
