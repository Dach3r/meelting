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

const steps = ["identity", "engine", "behavior"] as const;

export function AgentForm({ team, busy, errorMessage, onSubmit, onCancel }: Props) {
  const { t } = useI18n();
  const [step, setStep] = useState(0);
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
  const filled = (...values: string[]) => values.every((value) => value.trim() !== "");
  const complete = {
    identity: filled(draft.name, draft.role),
    engine: filled(draft.provider, draft.model),
    behavior: true,
  };
  const current = steps[step];
  const last = step === steps.length - 1;

  function caption(index: number) {
    if (index === step) return t("teams.agent.stepCurrent");
    if (index < step) return t("teams.agent.stepDone");
    return steps[index] === "behavior" ? t("teams.agent.stepOptional") : t("teams.agent.stepRequired");
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!last) {
      setStep(step + 1);
      return;
    }
    if (await onSubmit(draft)) onCancel();
  }

  return (
    <form onSubmit={submit} className="grid gap-6 p-6 sm:grid-cols-[13rem_1fr]">
      <ol aria-label={t("teams.newAgent")} className="grid content-start gap-1">
        {steps.map((key, index) => (
          <li
            key={key}
            aria-current={index === step ? "step" : undefined}
            className={`flex items-center gap-3 rounded-2xl p-3 ${index === step ? "bg-raised" : ""}`}
          >
            <span
              className={`grid size-7 shrink-0 place-items-center rounded-full text-xs font-semibold ${index === step ? "bg-ink text-surface" : index < step ? "bg-hi text-ink" : "border border-line text-muted"}`}
            >
              {index + 1}
            </span>
            <span className="grid min-w-0">
              <span className={`text-sm ${index === step ? "font-semibold text-ink" : "font-medium text-muted"}`}>
                {t(`teams.agent.${key}`)}
              </span>
              <span className="text-xs text-muted">{caption(index)}</span>
            </span>
          </li>
        ))}
      </ol>
      <div className="grid content-between gap-6">
        <div className="grid content-start gap-4">
          <h3 className="pl-1 text-lg font-bold text-ink">{t(`teams.agent.${current}Title`)}</h3>
          {current === "identity" && (
            <>
              <Field label={t("teams.agent.name")} value={draft.name} onChange={set("name")} maxLength={80} autoFocus />
              <Field label={t("teams.agent.role")} value={draft.role} onChange={set("role")} maxLength={80} />
            </>
          )}
          {current === "engine" && (
            <>
              <Field label={t("teams.agent.provider")} value={draft.provider} onChange={set("provider")} autoFocus />
              <Field label={t("teams.agent.model")} value={draft.model} onChange={set("model")} />
            </>
          )}
          {current === "behavior" && (
            <>
              <Field
                label={t("teams.agent.description")}
                value={draft.description}
                onChange={set("description")}
                maxLength={500}
                multiline
              />
              <Field
                label={t("teams.agent.instructions")}
                value={draft.instructions}
                onChange={set("instructions")}
                maxLength={10000}
                multiline
              />
            </>
          )}
          {errorMessage && <Alert message={errorMessage} />}
        </div>
        <div className="flex justify-between gap-3">
          {step === 0 ? (
            <Button type="button" variant="ghost" size="pill" onClick={onCancel}>
              {t("teams.cancel")}
            </Button>
          ) : (
            <Button type="button" variant="ghost" size="pill" onClick={() => setStep(step - 1)}>
              {t("teams.hire.back")}
            </Button>
          )}
          <Button type="submit" size="pill" disabled={busy || !complete[current]}>
            {last ? t("teams.agent.submit") : t("teams.hire.next")}
          </Button>
        </div>
      </div>
    </form>
  );
}
