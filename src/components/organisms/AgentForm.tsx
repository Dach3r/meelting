import { useId, useState, type FormEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { AgentDraft, Team } from "../../hooks/useTeams";
import { Button } from "../atoms/Button";
import { Field } from "../atoms/Field";
import { Alert } from "../molecules/Alert";

type Props = {
  teams: Team[];
  busy: boolean;
  errorMessage: string | null;
  onSubmit: (draft: AgentDraft) => Promise<boolean>;
  onCancel: () => void;
};

export function AgentForm({ teams, busy, errorMessage, onSubmit, onCancel }: Props) {
  const { t } = useI18n();
  const teamId = useId();
  const [draft, setDraft] = useState<AgentDraft>({
    team_id: teams[0].id,
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
    <form onSubmit={submit} className="grid gap-4 rounded-2xl border border-line bg-raised p-5">
      <div className="grid gap-2">
        <label htmlFor={teamId} className="text-sm font-medium text-muted">
          {t("teams.agent.team")}
        </label>
        <select
          id={teamId}
          value={draft.team_id}
          onChange={(event) => set("team_id")(event.target.value)}
          className="w-full rounded-xl border border-line bg-surface px-4 py-3 text-base text-ink outline-none focus:border-ink"
        >
          {teams.map((team) => (
            <option key={team.id} value={team.id}>
              {team.name}
            </option>
          ))}
        </select>
      </div>
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
