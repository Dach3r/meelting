import { useState, type FormEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { AgentDraft, Team } from "../../hooks/useTeams";
import { Button } from "../atoms/Button";
import { Field } from "../atoms/Field";
import { Alert } from "../molecules/Alert";

type Props = {
  team: Team;
  busy: boolean;
  errorMessage: string | null;
  onSubmit: (draft: AgentDraft) => Promise<boolean>;
  onCancel: () => void;
};

export function AgentForm({ team, busy, errorMessage, onSubmit, onCancel }: Props) {
  const { t } = useI18n();
  const [draft, setDraft] = useState<AgentDraft>({
    team_id: team.id,
    name: "",
    role: "",
    description: "",
    instructions: "",
    provider: "",
    model: "",
  });
  const set = (field: keyof AgentDraft) => (value: string) => setDraft({ ...draft, [field]: value });
  const complete = [draft.name, draft.role, draft.provider, draft.model].every((value) => value.trim() !== "");

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (await onSubmit(draft)) onCancel();
  }

  return (
    <form onSubmit={submit} className="grid gap-4 rounded-2xl border border-line bg-surface p-5">
      <div className="grid gap-4 sm:grid-cols-2">
        <Field label={t("teams.agent.name")} value={draft.name} onChange={set("name")} maxLength={80} autoFocus />
        <Field label={t("teams.agent.role")} value={draft.role} onChange={set("role")} maxLength={80} />
        <Field label={t("teams.agent.provider")} value={draft.provider} onChange={set("provider")} />
        <Field label={t("teams.agent.model")} value={draft.model} onChange={set("model")} />
      </div>
      <Field label={t("teams.agent.description")} value={draft.description} onChange={set("description")} maxLength={500} multiline />
      <Field label={t("teams.agent.instructions")} value={draft.instructions} onChange={set("instructions")} maxLength={10000} multiline />
      {errorMessage && <Alert message={errorMessage} />}
      <div className="flex justify-end gap-3">
        <Button type="button" variant="ghost" size="compact" onClick={onCancel}>
          {t("teams.cancel")}
        </Button>
        <Button type="submit" size="compact" disabled={busy || !complete}>
          {t("teams.agent.submit")}
        </Button>
      </div>
    </form>
  );
}
