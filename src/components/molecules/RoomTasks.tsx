import { useI18n } from "../../i18n/I18nProvider";
import type { Task } from "../../hooks/useTasks";
import { TaskBadges } from "./TaskBadges";

export function RoomTasks({ tasks }: { tasks: Task[] }) {
  const { t } = useI18n();
  if (tasks.length === 0) {
    return (
      <p className="flex-1 text-sm text-muted">{t("rooms.chat.noTasks")}</p>
    );
  }
  return (
    <ul className="grid flex-1 content-start gap-3 overflow-y-auto overscroll-contain">
      {tasks.map((task) => (
        <li
          key={task.id}
          className="grid gap-2 rounded-xl border border-line bg-shell p-4"
        >
          <TaskBadges status={task.status} priority={task.priority} />
          <p className="line-clamp-2 font-semibold text-ink">{task.title}</p>
        </li>
      ))}
    </ul>
  );
}
