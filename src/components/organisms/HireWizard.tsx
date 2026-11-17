import { useState, type FormEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { AgentTemplate } from "../../hooks/useAgentTemplates";
import type { AgentDraft, Team } from "../../hooks/useTeams";
import { Button } from "../atoms/Button";
import { Field } from "../atoms/Field";
import { Alert } from "../molecules/Alert";

type Props = {
  team: Team;
  template: AgentTemplate;
  busy: boolean;
  errorMessage: string | null;
  onSubmit: (draft: AgentDraft) => Promise<boolean>;
  onCancel: () => void;
};

const steps = ["name", "role", "description", "instructions", "engine"] as const;

export function HireWizard({ team, template, busy, errorMessage, onSubmit, onCancel }: Props) {
  const { t } = useI18n();
  const [step, setStep] = useState(0);
  const [draft, setDraft] = useState<AgentDraft>({
    team_id: team.id,
    template_key: template.key,
    name: template.name,
    role: template.role,
    description: template.description,
    instructions: template.instructions,
    provider: "",
    model: "",
  });
  const set = (field: keyof AgentDraft) => (value: string) => setDraft({ ...draft, [field]: value });
  const current = steps[step];
  const last = step === steps.length - 1;
  const filled = (...values: string[]) => values.every((value) => value.trim() !== "");
  const ready =
    current === "name"
      ? filled(draft.name)
      : current === "role"
        ? filled(draft.role)
        : current === "engine"
          ? filled(draft.provider, draft.model)
          : true;

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!last) {
      setStep(step + 1);
      return;
    }
    if (await onSubmit(draft)) onCancel();
    else setStep(0);
  }

  return (
    <form onSubmit={submit}>
      <div className="grid gap-4 px-6 py-5">
        <p className="pl-3 text-sm font-medium text-muted">
          {`${t("teams.hire.step")} ${step + 1} / ${steps.length}`}
        </p>
        {current === "name" && (
          <Field label={t("teams.agent.name")} value={draft.name} onChange={set("name")} maxLength={80} autoFocus />
        )}
        {current === "role" && (
          <Field label={t("teams.agent.role")} value={draft.role} onChange={set("role")} maxLength={80} autoFocus />
        )}
        {current === "description" && (
          <Field
            label={t("teams.agent.description")}
            value={draft.description}
            onChange={set("description")}
            maxLength={500}
            multiline
          />
        )}
        {current === "instructions" && (
          <Field
            label={t("teams.agent.instructions")}
            value={draft.instructions}
            onChange={set("instructions")}
            maxLength={10000}
            multiline
          />
        )}
        {current === "engine" && (
          <>
            <Field label={t("teams.agent.provider")} value={draft.provider} onChange={set("provider")} autoFocus />
            <Field label={t("teams.agent.model")} value={draft.model} onChange={set("model")} />
          </>
        )}
        {errorMessage && <Alert message={errorMessage} />}
      </div>
      <div className="flex justify-end gap-3 px-6 pb-6">
        {step === 0 ? (
          <Button type="button" variant="ghost" size="pill" onClick={onCancel}>
            {t("teams.cancel")}
          </Button>
        ) : (
          <Button type="button" variant="ghost" size="pill" onClick={() => setStep(step - 1)}>
            {t("teams.hire.back")}
          </Button>
        )}
        <Button type="submit" size="pill" disabled={busy || !ready}>
          {last ? t("teams.hire.submit") : t("teams.hire.next")}
        </Button>
      </div>
    </form>
  );
}
