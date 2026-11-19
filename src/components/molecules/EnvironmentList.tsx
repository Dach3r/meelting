import { FolderGit2 } from "lucide-react";
import type { ReactNode } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Environment } from "../../hooks/useEnvironments";

type Props = {
  label: string;
  environments: Environment[];
  selected: Set<string>;
  selectable: (environment: Environment) => boolean;
  focusedId: string;
  onFocus: (taskId: string) => void;
  onToggle: (taskId: string) => void;
  onToggleAll: () => void;
  aside: (environment: Environment) => ReactNode;
};

export function EnvironmentList({
  label,
  environments,
  selected,
  selectable,
  focusedId,
  onFocus,
  onToggle,
  onToggleAll,
  aside,
}: Props) {
  const { t } = useI18n();
  const candidates = environments.filter(selectable);
  const allSelected =
    candidates.length > 0 &&
    candidates.every((environment) => selected.has(environment.task_id));
  return (
    <div className="grid gap-2">
      <label className="flex cursor-pointer items-center gap-3 px-3 py-2 text-sm font-medium text-muted">
        <input
          type="checkbox"
          checked={allSelected}
          disabled={candidates.length === 0}
          onChange={onToggleAll}
        />
        {t("files.selectAll")}
      </label>
      <ul aria-label={label} className="grid gap-1">
        {environments.map((environment) => {
          const focused = environment.task_id === focusedId;
          return (
            <li
              key={environment.task_id}
              className={`flex min-w-0 items-center gap-3 rounded-2xl border px-3 py-1.5 ${focused ? "border-line bg-raised" : "border-transparent"}`}
            >
              <input
                type="checkbox"
                aria-label={environment.task_title}
                checked={selected.has(environment.task_id)}
                disabled={!selectable(environment)}
                onChange={() => onToggle(environment.task_id)}
              />
              <button
                type="button"
                aria-current={focused}
                onClick={() => onFocus(environment.task_id)}
                className="flex min-h-11 min-w-0 flex-1 cursor-pointer items-center gap-3 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
              >
                <FolderGit2
                  aria-hidden
                  className="size-5 shrink-0 text-muted"
                />
                <span className="grid min-w-0 flex-1 gap-0.5">
                  <span className="truncate font-semibold text-ink">
                    {environment.task_title}
                  </span>
                  <span className="truncate text-xs text-muted">
                    {environment.repository_name}
                  </span>
                </span>
              </button>
              <div className="shrink-0">{aside(environment)}</div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
