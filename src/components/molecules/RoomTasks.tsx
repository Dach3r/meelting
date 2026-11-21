import { useI18n } from "../../i18n/I18nProvider";
import type { Task } from "../../hooks/useTasks";
import { Button } from "../atoms/Button";
import { Alert } from "./Alert";
import { TaskBadges } from "./TaskBadges";

type Props = {
  tasks: Task[];
  busy: boolean;
  errorMessage: string | null;
  onPrepare: (taskId: string) => void;
  onOpenChanges: (taskId: string) => void;
};

export function RoomTasks({
  tasks,
  busy,
  errorMessage,
  onPrepare,
  onOpenChanges,
}: Props) {
  const { t } = useI18n();
  if (tasks.length === 0) {
    return (
      <p className="flex-1 text-sm text-muted">{t("rooms.chat.noTasks")}</p>
    );
  }
  return (
    <div className="grid flex-1 content-start gap-3 overflow-y-auto overscroll-contain">
      {errorMessage && <Alert message={errorMessage} />}
      <ul className="grid gap-3">
        {tasks.map((task) => {
          const worktree = task.worktree;
          const closed =
            task.status === "completed" || task.status === "canceled";
          const preparable =
            task.repository_id !== null &&
            task.base_ref !== null &&
            !worktree &&
            !closed;
          const reviewable =
            worktree?.status === "ready" && !worktree.trashed_at;
          return (
            <li
              key={task.id}
              className="grid gap-2 rounded-xl border border-line bg-shell p-4"
            >
              <TaskBadges status={task.status} priority={task.priority} />
              <p className="line-clamp-2 font-semibold text-ink">
                {task.title}
              </p>
              {worktree && (
                <p className="text-xs text-muted">
                  {t("tasks.environment.title")} ·{" "}
                  {worktree.trashed_at
                    ? t("tasks.environment.inTrash")
                    : t(`tasks.environment.status.${worktree.status}`)}
                </p>
              )}
              {preparable && (
                <Button
                  type="button"
                  size="compact"
                  variant="ghost"
                  className="justify-self-start"
                  disabled={busy}
                  onClick={() => onPrepare(task.id)}
                >
                  {t("tasks.environment.prepare")}
                </Button>
              )}
              {reviewable && (
                <Button
                  type="button"
                  size="compact"
                  variant="ghost"
                  className="justify-self-start"
                  onClick={() => onOpenChanges(task.id)}
                >
                  {t("tools.viewChanges")}
                </Button>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
