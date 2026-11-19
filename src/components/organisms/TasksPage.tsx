import { FolderGit2, Plus } from "lucide-react";
import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { translateError } from "../../i18n/translate";
import { useRepositories } from "../../hooks/useRepositories";
import { taskStatuses, useTasks, type Task } from "../../hooks/useTasks";
import type { Agent } from "../../hooks/useTeams";
import { Button } from "../atoms/Button";
import { Select } from "../atoms/Select";
import { Alert } from "../molecules/Alert";
import { Modal } from "../molecules/Modal";
import { RepositoryList } from "../molecules/RepositoryList";
import { TaskCard } from "../molecules/TaskCard";
import { TaskDetail } from "./TaskDetail";
import { TaskDialog } from "./TaskDialog";

const ALL = "";
const NONE = "none";

export function TasksPage({ agents }: { agents: Agent[] }) {
  const { t, locale } = useI18n();
  const {
    tasks,
    errorKey,
    actionErrorKey,
    busy,
    replace,
    changeStatus,
    prepareEnvironment,
    discardEnvironment,
    removeEnvironment,
    checkRemoval,
    pickFiles,
    copyFiles,
    deleteTask,
    clearActionError,
  } = useTasks();
  const repositories = useRepositories();
  const [dialog, setDialog] = useState<"new" | "edit" | "repositories" | null>(
    null,
  );
  const [openId, setOpenId] = useState<string | null>(null);
  const [statusFilter, setStatusFilter] = useState(ALL);
  const [repositoryFilter, setRepositoryFilter] = useState(ALL);
  const [agentFilter, setAgentFilter] = useState(ALL);
  const repositoryList = repositories.repositories ?? [];
  const repositoryOf = (task: Task) =>
    repositoryList.find((repository) => repository.id === task.repository_id);
  const actionErrorMessage = actionErrorKey
    ? translateError(locale, actionErrorKey)
    : null;
  const openTask = tasks?.find((task) => task.id === openId);

  function saved(task: Task) {
    replace(task);
    setDialog(null);
    setOpenId(task.id);
  }

  function closeRepositories() {
    repositories.clearActionError();
    setDialog(null);
  }

  const dialogs = (
    <>
      {dialog === "new" && (
        <TaskDialog
          agents={agents}
          onSaved={saved}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog === "edit" && openTask && (
        <TaskDialog
          agents={agents}
          task={openTask}
          onSaved={saved}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog === "repositories" && (
        <Modal title={t("tasks.repositories.title")} onClose={closeRepositories}>
          <RepositoryList
            repositories={repositoryList}
            busy={repositories.busy}
            errorMessage={
              repositories.actionErrorKey
                ? translateError(locale, repositories.actionErrorKey)
                : null
            }
            onRegister={repositories.registerRepository}
            onUnregister={repositories.unregisterRepository}
          />
        </Modal>
      )}
    </>
  );

  if (openTask) {
    return (
      <>
        {dialogs}
        <TaskDetail
          task={openTask}
          repository={repositoryOf(openTask)}
          agents={agents}
          busy={busy}
          errorMessage={actionErrorMessage}
          onBack={() => {
            clearActionError();
            setOpenId(null);
          }}
          onEdit={() => setDialog("edit")}
          onChangeStatus={(status) => changeStatus(openTask.id, status)}
          onPrepareEnvironment={() => prepareEnvironment(openTask.id)}
          onDiscardEnvironment={() => discardEnvironment(openTask.id)}
          onCheckRemoval={() => checkRemoval(openTask.id)}
          onRemoveEnvironment={() => removeEnvironment(openTask.id)}
          onPickFiles={() => pickFiles(openTask.id)}
          onCopyFiles={(selectionId, acceptSecrets) =>
            copyFiles(openTask.id, selectionId, acceptSecrets)
          }
          onDelete={async () => {
            if (await deleteTask(openTask.id)) setOpenId(null);
          }}
          onClearError={clearActionError}
        />
      </>
    );
  }

  const visible = (tasks ?? []).filter(
    (task) =>
      (statusFilter === ALL || task.status === statusFilter) &&
      (repositoryFilter === ALL ||
        (repositoryFilter === NONE
          ? task.repository_id === null
          : task.repository_id === repositoryFilter)) &&
      (agentFilter === ALL || task.agent_ids.includes(agentFilter)),
  );
  const all = { value: ALL, label: t("tasks.filters.all") };

  return (
    <div className="flex w-full flex-col gap-6">
      {dialogs}
      <header className="flex flex-wrap items-center justify-between gap-4">
        <h1 className="text-2xl font-bold text-ink">{t("tasks.title")}</h1>
        <div className="flex gap-2">
          <Button
            type="button"
            variant="ghost"
            size="pill"
            onClick={() => setDialog("repositories")}
          >
            <FolderGit2 aria-hidden className="size-4" />
            {t("tasks.repositories.action")}
          </Button>
          <Button type="button" size="pill" onClick={() => setDialog("new")}>
            <Plus aria-hidden className="size-4" />
            {t("tasks.new")}
          </Button>
        </div>
      </header>
      {errorKey && <Alert message={translateError(locale, errorKey)} />}
      {repositories.errorKey && (
        <Alert message={translateError(locale, repositories.errorKey)} />
      )}
      {tasks && tasks.length === 0 && (
        <div className="grid gap-1 py-16 text-center">
          <h2 className="text-xl font-bold">{t("tasks.empty.title")}</h2>
          <p className="text-muted">{t("tasks.empty.body")}</p>
        </div>
      )}
      {tasks && tasks.length > 0 && (
        <>
          <section
            aria-label={t("tasks.filters.label")}
            className="grid gap-4 sm:grid-cols-3"
          >
            <Select
              label={t("tasks.filters.status")}
              value={statusFilter}
              onChange={setStatusFilter}
              options={[
                all,
                ...taskStatuses.map((status) => ({
                  value: status,
                  label: t(`tasks.status.${status}`),
                })),
              ]}
            />
            <Select
              label={t("tasks.filters.repository")}
              value={repositoryFilter}
              onChange={setRepositoryFilter}
              options={[
                all,
                { value: NONE, label: t("tasks.filters.noRepository") },
                ...repositoryList.map((repository) => ({
                  value: repository.id,
                  label: repository.name,
                })),
              ]}
            />
            <Select
              label={t("tasks.filters.agent")}
              value={agentFilter}
              onChange={setAgentFilter}
              options={[
                all,
                ...agents.map((agent) => ({
                  value: agent.id,
                  label: agent.name,
                })),
              ]}
            />
          </section>
          {visible.length === 0 ? (
            <p className="py-10 text-center text-muted">
              {t("tasks.filters.noMatch")}
            </p>
          ) : (
            <ul
              aria-label={t("tasks.title")}
              className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3"
            >
              {visible.map((task) => (
                <TaskCard
                  key={task.id}
                  task={task}
                  repository={repositoryOf(task)}
                  agents={agents}
                  onOpen={() => {
                    clearActionError();
                    setOpenId(task.id);
                  }}
                />
              ))}
            </ul>
          )}
        </>
      )}
    </div>
  );
}
