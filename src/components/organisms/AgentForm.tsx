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
    <form onSubmit={submit} className="grid min-h-[38rem] sm:grid-cols-[17rem_1fr]">
      <div className="grid content-start bg-raised/50 px-4 py-6 sm:px-5 sm:py-10">
        <ol aria-label={t("teams.newAgent")} className="grid gap-3">
          {steps.map((key, index) => (
            <li
              key={key}
              aria-current={index === step ? "step" : undefined}
              className={`flex items-center gap-4 rounded-2xl px-4 py-4 ${index === step ? "bg-hi" : ""}`}
            >
              <span
                className={`grid size-7 shrink-0 place-items-center rounded-full text-xs font-semibold ${index === step ? "bg-ink text-surface" : index < step ? "bg-hi text-ink" : "border border-muted text-muted"}`}
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
      </div>
      <div className="flex flex-col gap-10 p-6 sm:p-10">
        <div className="grid gap-2">
          <h3 className="text-2xl font-bold text-ink">{t(`teams.agent.${current}Title`)}</h3>
          <p className="text-sm text-muted">{t(`teams.agent.${current}Subtitle`)}</p>
        </div>
        <div className="grid flex-1 content-start gap-7">
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
        <div className="flex items-center justify-between gap-3">
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
