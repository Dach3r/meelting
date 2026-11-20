import { Search } from "lucide-react";
import { useId, useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { translateError } from "../../i18n/translate";
import { useTasks, type Task, type TaskOrigin } from "../../hooks/useTasks";
import { fieldClasses } from "../atoms/Field";
import { Alert } from "../molecules/Alert";
import { TaskBadges } from "../molecules/TaskBadges";

type Props = {
  origin: TaskOrigin;
  busy: boolean;
  errorMessage: string | null;
  onLink: (task: Task) => void;
};

const alreadyLinked = (task: Task, origin: TaskOrigin) =>
  [
    { room_id: task.origin_room_id, message_id: task.origin_message_id },
    ...task.linked_rooms,
  ].some(
    (link) =>
      link.room_id === origin.room_id && link.message_id === origin.message_id,
  );

export function TaskLinkPanel({ origin, busy, errorMessage, onLink }: Props) {
  const { t, locale } = useI18n();
  const { tasks, errorKey } = useTasks();
  const id = useId();
  const [query, setQuery] = useState("");
  const needle = query.trim().toLowerCase();
  const options = (tasks ?? []).filter(
    (task) =>
      !alreadyLinked(task, origin) && task.title.toLowerCase().includes(needle),
  );

  return (
    <div className="grid gap-4 p-6 sm:px-10 sm:pb-8">
      <p className="text-xs text-muted">{t("tasks.link.linkHint")}</p>
      {(errorKey || errorMessage) && (
        <Alert message={errorMessage ?? translateError(locale, errorKey!)} />
      )}
      <div className="relative">
        <Search
          aria-hidden
          className="pointer-events-none absolute top-1/2 left-4 size-5 -translate-y-1/2 text-muted"
        />
        <input
          id={id}
          aria-label={t("tasks.link.search")}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder={t("tasks.link.search")}
          className={`${fieldClasses} pl-12`}
        />
      </div>
      {tasks &&
        (options.length === 0 ? (
          <p className="text-sm text-muted">{t("tasks.link.none")}</p>
        ) : (
          <ul
            aria-label={t("tasks.link.available")}
            className="max-h-72 overflow-y-auto overscroll-contain rounded-2xl border border-line"
          >
            {options.map((task) => (
              <li
                key={task.id}
                className="border-b border-line/60 last:border-b-0"
              >
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => onLink(task)}
                  className="flex w-full cursor-pointer flex-col gap-1.5 px-3.5 py-2.5 text-left hover:bg-hi/50 disabled:cursor-not-allowed disabled:opacity-60"
                >
                  <span className="truncate text-sm font-semibold text-ink">
                    {task.title}
                  </span>
                  <TaskBadges status={task.status} priority={task.priority} />
                </button>
              </li>
            ))}
          </ul>
        ))}
    </div>
  );
}
