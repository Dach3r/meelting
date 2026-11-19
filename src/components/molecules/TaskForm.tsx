import { ListChecks } from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type {
  Repository,
  RepositoryBranches,
} from "../../hooks/useRepositories";
import type { TaskDraft, TaskPriority } from "../../hooks/useTasks";
import type { Agent } from "../../hooks/useTeams";
import { translateError } from "../../i18n/translate";
import { Button } from "../atoms/Button";
import { Field } from "../atoms/Field";
import { Select } from "../atoms/Select";
import { AgentPicker } from "./AgentPicker";
import { Alert } from "./Alert";
import { SegmentedControl } from "./SegmentedControl";

const priorities: TaskPriority[] = ["low", "medium", "high"];

type Props = {
  agents: Agent[];
  repositories: Repository[];
  initial: TaskDraft;
  editing: boolean;
  busy: boolean;
  errorMessage: string | null;
  onLoadBranches: (repositoryId: string) => Promise<RepositoryBranches>;
  onSubmit: (draft: TaskDraft) => void;
  onCancel: () => void;
};

export function TaskForm({
  agents,
  repositories,
  initial,
  editing,
  busy,
  errorMessage,
  onLoadBranches,
  onSubmit,
  onCancel,
}: Props) {
  const { t, locale } = useI18n();
  const [draft, setDraft] = useState(initial);
  const [branches, setBranches] = useState<string[] | null>(null);
  const [branchErrorKey, setBranchErrorKey] = useState<string | null>(null);
  const repositoryId = draft.repository_id;

  useEffect(() => {
    if (!repositoryId) return;
    let current = true;
    setBranches(null);
    setBranchErrorKey(null);
    onLoadBranches(repositoryId)
      .then((loaded) => {
        if (!current) return;
        setBranches(loaded.branches);
        setDraft((previous) =>
          previous.base_ref && loaded.branches.includes(previous.base_ref)
            ? previous
            : { ...previous, base_ref: loaded.default_branch },
        );
      })
      .catch((error) => {
        if (current) setBranchErrorKey(String(error));
      });
    return () => {
      current = false;
    };
  }, [repositoryId, onLoadBranches]);

  function update(changes: Partial<TaskDraft>) {
    setDraft((previous) => ({ ...previous, ...changes }));
  }

  function changeRepository(value: string) {
    update({ repository_id: value || null, base_ref: null });
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    onSubmit(draft);
  }

  const branchOptions = (branches ?? []).map((branch) => ({
    value: branch,
    label: branch,
  }));

  return (
    <form onSubmit={submit} className="grid gap-5 p-6 sm:px-10 sm:pb-8">
      <Field
        label={t("tasks.form.title")}
        value={draft.title}
        onChange={(title) => update({ title })}
        maxLength={120}
        icon={ListChecks}
        autoFocus
      />
      <Field
        label={t("tasks.form.description")}
        hint={t("tasks.form.descriptionHint")}
        value={draft.description}
        onChange={(description) => update({ description })}
        maxLength={10000}
        multiline
      />
      <div className="grid gap-2">
        <span className="text-sm font-medium text-muted">
          {t("tasks.priority.label")}
        </span>
        <SegmentedControl
          label={t("tasks.priority.label")}
          value={draft.priority}
          options={priorities.map((priority) => ({
            value: priority,
            label: t(`tasks.priority.${priority}`),
          }))}
          onChange={(priority) => update({ priority })}
        />
      </div>
      <div className="grid gap-5 sm:grid-cols-2">
        <Select
          label={t("tasks.form.repository")}
          hint={t("tasks.form.repositoryHint")}
          value={draft.repository_id ?? ""}
          options={[
            { value: "", label: t("tasks.form.noRepository") },
            ...repositories.map((repository) => ({
              value: repository.id,
              label: repository.name,
            })),
          ]}
          onChange={changeRepository}
        />
        {draft.repository_id && (
          <Select
            label={t("tasks.form.baseRef")}
            hint={t("tasks.form.baseRefHint")}
            value={draft.base_ref ?? ""}
            disabled={branches === null}
            options={
              branches === null
                ? [{ value: "", label: t("tasks.form.loadingBranches") }]
                : branchOptions
            }
            onChange={(base_ref) => update({ base_ref: base_ref || null })}
          />
        )}
      </div>
      {branchErrorKey && (
        <Alert message={translateError(locale, branchErrorKey)} />
      )}
      <AgentPicker
        agents={agents}
        value={draft.agent_ids}
        onChange={(agent_ids) => update({ agent_ids })}
        hint={t("tasks.form.agentsHint")}
      />
      {errorMessage && <Alert message={errorMessage} />}
      <div className="flex justify-end gap-3">
        <Button type="button" variant="ghost" size="pill" onClick={onCancel}>
          {t("tasks.cancel")}
        </Button>
        <Button
          type="submit"
          size="pill"
          disabled={
            busy ||
            draft.title.trim() === "" ||
            (draft.repository_id !== null && branches === null && !branchErrorKey)
          }
        >
          {editing ? t("tasks.form.submitEdit") : t("tasks.form.submitNew")}
        </Button>
      </div>
    </form>
  );
}
