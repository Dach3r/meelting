import { FolderGit2 } from "lucide-react";
import type { ReactNode } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Environment } from "../../hooks/useEnvironments";

type Props = {
  label: string;
  environments: Environment[];
  selected: Set<string>;
  selectable: (environment: Environment) => boolean;
  onToggle: (taskId: string) => void;
  onToggleAll: () => void;
  aside: (environment: Environment) => ReactNode;
};

export function EnvironmentList({
  label,
  environments,
  selected,
  selectable,
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
    <div className="grid overflow-hidden rounded-2xl border border-line">
      <label className="flex cursor-pointer items-center gap-3 border-b border-line bg-shell px-4 py-3 text-sm font-medium text-muted">
        <input
          type="checkbox"
          checked={allSelected}
          disabled={candidates.length === 0}
          onChange={onToggleAll}
        />
        {t("files.selectAll")}
      </label>
      <ul aria-label={label} className="grid">
        {environments.map((environment) => {
          const { worktree } = environment;
          return (
            <li
              key={environment.task_id}
              className="flex min-w-0 items-center gap-3 border-b border-line/60 px-4 py-3 last:border-b-0"
            >
              <input
                type="checkbox"
                aria-label={environment.task_title}
                checked={selected.has(environment.task_id)}
                disabled={!selectable(environment)}
                onChange={() => onToggle(environment.task_id)}
              />
              <FolderGit2 aria-hidden className="size-5 shrink-0 text-muted" />
              <div className="grid min-w-0 flex-1 gap-0.5">
                <span className="truncate font-semibold text-ink">
                  {environment.task_title}
                </span>
                <span className="truncate text-xs text-muted">
                  {environment.repository_name} ·{" "}
                  <code>{worktree.branch_name}</code>
                </span>
                <code
                  className="truncate text-xs text-muted select-text"
                  title={worktree.trash_path ?? worktree.path}
                >
                  {worktree.trash_path ?? worktree.path}
                </code>
              </div>
              <div className="shrink-0">{aside(environment)}</div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
