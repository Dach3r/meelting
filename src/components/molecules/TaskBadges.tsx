import { useI18n } from "../../i18n/I18nProvider";
import type { TaskPriority, TaskStatus } from "../../hooks/useTasks";

const statusTones: Record<TaskStatus, string> = {
  pending: "border-line text-muted",
  in_progress: "border-ink text-ink",
  blocked: "border-error text-error",
  completed: "border-line bg-pill text-ink",
  canceled: "border-line text-muted line-through",
};

export function TaskBadges({
  status,
  priority,
}: {
  status: TaskStatus;
  priority: TaskPriority;
}) {
  const { t } = useI18n();
  return (
    <p className="flex flex-wrap items-center gap-2 text-xs font-semibold">
      <span
        className={`rounded-full border px-2.5 py-0.5 ${statusTones[status]}`}
      >
        {t(`tasks.status.${status}`)}
      </span>
      <span className="rounded-full border border-line px-2.5 py-0.5 text-muted">
        {t("tasks.priority.label")}: {t(`tasks.priority.${priority}`)}
      </span>
    </p>
  );
}
