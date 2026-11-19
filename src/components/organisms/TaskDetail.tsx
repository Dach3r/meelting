import { ArrowLeft, Pencil, Trash2 } from "lucide-react";
import { useState, type ReactNode } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { formatDateTime, translateWith } from "../../i18n/translate";
import type { Repository } from "../../hooks/useRepositories";
import {
  nextStatuses,
  type RemovalCheck,
  type Task,
  type TaskStatus,
} from "../../hooks/useTasks";
import type { Agent } from "../../hooks/useTeams";
import { Avatar } from "../atoms/Avatar";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { Modal } from "../molecules/Modal";
import { TaskBadges } from "../molecules/TaskBadges";
import { TaskEnvironment } from "../molecules/TaskEnvironment";

type Props = {
  task: Task;
  repository: Repository | undefined;
  agents: Agent[];
  busy: boolean;
  errorMessage: string | null;
  onBack: () => void;
  onEdit: () => void;
  onChangeStatus: (status: TaskStatus) => void;
  onPrepareEnvironment: () => void;
  onDiscardEnvironment: () => void;
  onCheckRemoval: () => Promise<RemovalCheck | null>;
  onRemoveEnvironment: () => Promise<void>;
  onDelete: () => Promise<void>;
  onClearError: () => void;
};

export function TaskDetail({
  task,
  repository,
  agents,
  busy,
  errorMessage,
  onBack,
  onEdit,
  onChangeStatus,
  onPrepareEnvironment,
  onDiscardEnvironment,
  onCheckRemoval,
  onRemoveEnvironment,
  onDelete,
  onClearError,
}: Props) {
  const { t, locale } = useI18n();
  const [deleting, setDeleting] = useState(false);
  const assigned = task.agent_ids.flatMap((agentId) =>
    agents.filter((agent) => agent.id === agentId),
  );

  function actionLabel(status: TaskStatus) {
    const reopening =
      task.status === "completed" || task.status === "canceled";
    return reopening ? t("tasks.actions.reopen") : t(`tasks.actions.${status}`);
  }

  function closeDelete() {
    onClearError();
    setDeleting(false);
  }

  return (
    <div className="flex w-full max-w-4xl flex-col gap-6">
      {deleting && (
        <Modal title={t("tasks.deleteDialog.title")} onClose={closeDelete}>
          <div className="grid gap-5 p-6">
            <p className="text-muted">{t("tasks.deleteDialog.body")}</p>
            {errorMessage && <Alert message={errorMessage} />}
            <div className="flex justify-end gap-3">
              <Button
                type="button"
                variant="ghost"
                size="compact"
                onClick={closeDelete}
              >
                {t("tasks.cancel")}
              </Button>
              <Button
                type="button"
                variant="destructive"
                size="compact"
                disabled={busy}
                onClick={onDelete}
              >
                {t("tasks.deleteDialog.confirm")}
              </Button>
            </div>
          </div>
        </Modal>
      )}
      <Button
        type="button"
        variant="ghost"
        size="compact"
        className="self-start"
        onClick={onBack}
      >
        <ArrowLeft aria-hidden className="size-4" />
        {t("tasks.detail.back")}
      </Button>
      <header className="grid gap-3">
        <TaskBadges status={task.status} priority={task.priority} />
        <div className="flex flex-wrap items-start justify-between gap-3">
          <h1 className="min-w-0 flex-1 text-2xl font-bold break-words text-ink">
            {task.title}
          </h1>
          <div className="flex gap-2">
            <Button type="button" variant="ghost" size="compact" onClick={onEdit}>
              <Pencil aria-hidden className="size-4" />
              {t("tasks.detail.edit")}
            </Button>
            <Button
              type="button"
              variant="danger"
              size="compact"
              onClick={() => {
                onClearError();
                setDeleting(true);
              }}
            >
              <Trash2 aria-hidden className="size-4" />
              {t("tasks.detail.delete")}
            </Button>
          </div>
        </div>
      </header>
      <nav aria-label={t("tasks.actions.label")} className="flex flex-wrap gap-2">
        {nextStatuses[task.status].map((status) => (
          <Button
            key={status}
            type="button"
            variant={status === "completed" ? "primary" : "ghost"}
            size="pill"
            disabled={busy}
            className={status === "completed" ? "" : "border-line"}
            onClick={() => onChangeStatus(status)}
          >
            {actionLabel(status)}
          </Button>
        ))}
      </nav>
      {errorMessage && !deleting && <Alert message={errorMessage} />}
      <section
        aria-label={t("tasks.detail.information")}
        className="grid gap-5 rounded-2xl border border-line bg-shell p-6"
      >
        <p className="whitespace-pre-wrap break-words text-ink select-text">
          {task.description || (
            <span className="text-muted">{t("tasks.detail.noDescription")}</span>
          )}
        </p>
        <dl className="grid gap-4 sm:grid-cols-2">
          <Item label={t("tasks.detail.agents")}>
            {assigned.length === 0 ? (
              <span className="text-muted">{t("tasks.detail.noAgents")}</span>
            ) : (
              <ul className="flex flex-wrap gap-2">
                {assigned.map((agent) => (
                  <li
                    key={agent.id}
                    className="flex items-center gap-2 rounded-full border border-line bg-raised py-1 pr-3 pl-1 text-sm font-semibold"
                  >
                    <Avatar
                      seed={agent.name}
                      label={agent.name}
                      className="size-6 rounded-full text-xs"
                    />
                    {agent.name}
                  </li>
                ))}
              </ul>
            )}
          </Item>
          <Item label={t("tasks.detail.repository")}>
            {repository ? (
              <span title={repository.path}>{repository.name}</span>
            ) : (
              <span className="text-muted">
                {t("tasks.detail.noRepository")}
              </span>
            )}
          </Item>
          {task.base_ref && (
            <Item label={t("tasks.detail.baseRef")}>
              <code className="text-sm">{task.base_ref}</code>
            </Item>
          )}
          {task.origin_room_name && (
            <Item label={t("tasks.detail.origin")}>
              {translateWith(
                locale,
                task.origin_message_id
                  ? "tasks.detail.originMessage"
                  : "tasks.detail.originRoom",
                { name: task.origin_room_name },
              )}
            </Item>
          )}
          <Item label={t("tasks.detail.created")}>
            {formatDateTime(locale, task.created_at)}
          </Item>
          {task.completed_at && (
            <Item label={t("tasks.detail.completed")}>
              {formatDateTime(locale, task.completed_at)}
            </Item>
          )}
        </dl>
      </section>
      {task.repository_id && (
        <TaskEnvironment
          task={task}
          busy={busy}
          onPrepare={onPrepareEnvironment}
          onDiscard={onDiscardEnvironment}
          onCheckRemoval={onCheckRemoval}
          onRemove={onRemoveEnvironment}
        />
      )}
    </div>
  );
}

function Item({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid min-w-0 gap-1">
      <dt className="text-xs font-medium text-muted">{label}</dt>
      <dd className="min-w-0 break-words text-ink">{children}</dd>
    </div>
  );
}
