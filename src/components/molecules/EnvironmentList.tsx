import type { ReactNode } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Environment } from "../../hooks/useEnvironments";
import { FolderTileIcon } from "../atoms/Icons";

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

const checkbox =
  "size-[1.125rem] shrink-0 cursor-pointer accent-error disabled:cursor-not-allowed";

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
    <div className="grid gap-1">
      <label className="flex cursor-pointer items-center gap-3 px-4 py-2 text-sm font-medium text-muted">
        <input
          type="checkbox"
          className={checkbox}
          checked={allSelected}
          disabled={candidates.length === 0}
          onChange={onToggleAll}
        />
        {t("files.selectAll")}
      </label>
      <ul
        aria-label={label}
        className="grid grid-cols-[repeat(auto-fill,minmax(13rem,1fr))] gap-3"
      >
        {environments.map((environment) => {
          const focused = environment.task_id === focusedId;
          return (
            <li
              key={environment.task_id}
              className={`relative grid min-w-0 rounded-3xl transition-colors ${focused ? "bg-hi ring-1 ring-error/60" : "bg-raised hover:bg-pill-hover"}`}
            >
              <input
                type="checkbox"
                className={`${checkbox} absolute top-4 left-4 z-10`}
                aria-label={environment.task_title}
                checked={selected.has(environment.task_id)}
                disabled={!selectable(environment)}
                onChange={() => onToggle(environment.task_id)}
              />
              <button
                type="button"
                aria-current={focused}
                onClick={() => onFocus(environment.task_id)}
                className="grid min-w-0 cursor-pointer content-start gap-4 rounded-3xl px-5 pt-5 pb-5 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
              >
                <span
                  className={`grid h-28 place-items-center ${focused ? "text-error" : "text-muted"}`}
                >
                  <FolderTileIcon />
                </span>
                <span className="grid min-w-0 gap-2">
                  <span className="line-clamp-2 min-h-9 text-sm leading-[1.3] font-semibold text-ink">
                    {environment.task_title}
                  </span>
                  <span className="flex min-w-0 items-center gap-2 text-xs text-muted">
                    <span className="truncate">
                      {environment.repository_name}
                    </span>
                    <span aria-hidden>·</span>
                    {aside(environment)}
                  </span>
                  <code className="truncate text-[0.6875rem] text-muted">
                    {environment.worktree.branch_name}
                  </code>
                </span>
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
