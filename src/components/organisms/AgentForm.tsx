import { useState, type FormEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { AgentDraft, Team } from "../../hooks/useTeams";
import { Button } from "../atoms/Button";
import { Field } from "../atoms/Field";
import { Alert } from "../molecules/Alert";
import { SegmentedControl } from "../molecules/SegmentedControl";

type Props = {
  team: Team;
  busy: boolean;
  errorMessage: string | null;
  onSubmit: (draft: AgentDraft) => Promise<boolean>;
  onCancel: () => void;
};

type Tab = "identity" | "engine" | "behavior";

export function AgentForm({ team, busy, errorMessage, onSubmit, onCancel }: Props) {
  const { t } = useI18n();
  const [tab, setTab] = useState<Tab>("identity");
  const [draft, setDraft] = useState<AgentDraft>({
    team_id: team.id,
    template_key: null,
    name: "",
    role: "",
    description: "",
    instructions: "",
    provider: "",
    model: "",
  });
  const set = (field: keyof AgentDraft) => (value: string) => setDraft({ ...draft, [field]: value });
  const missing = (...values: string[]) => values.some((value) => value.trim() === "");
  const complete = [draft.name, draft.role, draft.provider, draft.model].every((value) => value.trim() !== "");

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (await onSubmit(draft)) onCancel();
  }

  return (
    <form onSubmit={submit}>
      <div className="grid gap-4 px-6 py-5">
        <SegmentedControl
          label={t("teams.newAgent")}
          options={[
            { value: "identity", label: t("teams.agent.identity"), pending: missing(draft.name, draft.role) },
            { value: "engine", label: t("teams.agent.engine"), pending: missing(draft.provider, draft.model) },
            { value: "behavior", label: t("teams.agent.behavior") },
          ]}
          value={tab}
          onChange={setTab}
        />
        {tab === "identity" && (
          <>
            <Field label={t("teams.agent.name")} value={draft.name} onChange={set("name")} maxLength={80} autoFocus />
            <Field label={t("teams.agent.role")} value={draft.role} onChange={set("role")} maxLength={80} />
          </>
        )}
        {tab === "engine" && (
          <>
            <Field label={t("teams.agent.provider")} value={draft.provider} onChange={set("provider")} autoFocus />
            <Field label={t("teams.agent.model")} value={draft.model} onChange={set("model")} />
          </>
        )}
        {tab === "behavior" && (
          <>
            <Field label={t("teams.agent.description")} value={draft.description} onChange={set("description")} maxLength={500} multiline />
            <Field label={t("teams.agent.instructions")} value={draft.instructions} onChange={set("instructions")} maxLength={10000} multiline />
          </>
        )}
        {errorMessage && <Alert message={errorMessage} />}
      </div>
      <div className="flex justify-end gap-3 px-6 pb-6">
        <Button type="button" variant="ghost" size="pill" onClick={onCancel}>
          {t("teams.cancel")}
        </Button>
        <Button type="submit" size="pill" disabled={busy || !complete}>
          {t("teams.agent.submit")}
        </Button>
      </div>
    </form>
  );
}
