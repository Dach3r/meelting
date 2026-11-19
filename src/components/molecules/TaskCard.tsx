import { FolderGit2 } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Repository } from "../../hooks/useRepositories";
import type { Task } from "../../hooks/useTasks";
import type { Agent } from "../../hooks/useTeams";
import { Avatar } from "../atoms/Avatar";
import { TaskBadges } from "./TaskBadges";

const SEATS = 4;

type Props = {
  task: Task;
  repository: Repository | undefined;
  agents: Agent[];
  onOpen: () => void;
};

export function TaskCard({ task, repository, agents, onOpen }: Props) {
  const { t } = useI18n();
  const assigned = task.agent_ids.flatMap((agentId) =>
    agents.filter((agent) => agent.id === agentId),
  );
  return (
    <li>
      <button
        type="button"
        onClick={onOpen}
        className="flex w-full cursor-pointer flex-col gap-3 rounded-[20px] border border-line bg-shell p-5 text-left transition-colors hover:bg-raised focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
      >
        <TaskBadges status={task.status} priority={task.priority} />
        <h3 className="line-clamp-2 text-lg font-semibold text-ink">
          {task.title}
        </h3>
        <div className="flex min-h-8 items-center justify-between gap-3 text-xs text-muted">
          <span className="flex min-w-0 items-center gap-1.5">
            <FolderGit2 aria-hidden className="size-4 shrink-0" />
            <span className="truncate">
              {repository?.name ?? t("tasks.detail.noRepository")}
            </span>
          </span>
          {assigned.length > 0 && (
            <span
              className="flex shrink-0 items-center"
              title={assigned.map((agent) => agent.name).join(", ")}
            >
              {assigned.slice(0, SEATS).map((agent) => (
                <Avatar
                  key={agent.id}
                  seed={agent.name}
                  label={agent.name}
                  className="-ml-1.5 size-7 rounded-full border-2 border-shell text-xs first:ml-0"
                />
              ))}
              {assigned.length > SEATS && (
                <span className="ml-1.5 font-medium">
                  +{assigned.length - SEATS}
                </span>
              )}
            </span>
          )}
        </div>
      </button>
    </li>
  );
}
