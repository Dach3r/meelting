import type { ReactNode } from "react";
import type { Environment } from "../../hooks/useEnvironments";
import { FolderTileIcon } from "../atoms/Icons";

type Props = {
  label: string;
  environments: Environment[];
  focusedId: string | null;
  onFocus: (taskId: string) => void;
  aside: (environment: Environment) => ReactNode;
};

export function EnvironmentList({
  label,
  environments,
  focusedId,
  onFocus,
  aside,
}: Props) {
  return (
    <div className="grid gap-1">
      <ul
        aria-label={label}
        className="grid grid-cols-[repeat(auto-fill,minmax(13rem,1fr))] gap-3"
      >
        {environments.map((environment) => {
          const focused = environment.task_id === focusedId;
          return (
            <li
              key={environment.task_id}
              className={`relative grid min-w-0 rounded-3xl transition-colors ${focused ? "bg-hi ring-1 ring-ink/40" : "bg-raised hover:bg-pill-hover"}`}
            >
              <button
                type="button"
                aria-current={focused}
                onClick={() => onFocus(environment.task_id)}
                className="grid min-w-0 cursor-pointer content-start gap-4 rounded-3xl px-5 pt-5 pb-5 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
              >
                <span
                  className={`grid h-28 place-items-center ${focused ? "text-ink" : "text-muted"}`}
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
