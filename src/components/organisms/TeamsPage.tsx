import { Bot, Plus } from "lucide-react";
import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { AgentDraft, Team } from "../../hooks/useTeams";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { TeamForm } from "../molecules/TeamForm";
import { AgentForm } from "./AgentForm";

type Props = {
  teams: Team[];
  errorMessage: string | null;
  actionErrorMessage: string | null;
  busy: boolean;
  onCreateTeam: (name: string) => Promise<boolean>;
  onCreateAgent: (draft: AgentDraft) => Promise<boolean>;
  onClearActionError: () => void;
};

export function TeamsPage({
  teams,
  errorMessage,
  actionErrorMessage,
  busy,
  onCreateTeam,
  onCreateAgent,
  onClearActionError,
}: Props) {
  const { t } = useI18n();
  const [form, setForm] = useState<{ kind: "team" } | { kind: "agent"; teamId: string } | null>(null);

  function open(next: NonNullable<typeof form>) {
    onClearActionError();
    setForm(next);
  }

  function close() {
    onClearActionError();
    setForm(null);
  }

  if (errorMessage) {
    return (
      <div className="mx-auto w-full max-w-4xl">
        <Alert message={errorMessage} />
      </div>
    );
  }

  return (
    <div className="mx-auto grid w-full max-w-4xl content-start gap-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-bold">{t("teams.title")}</h1>
        <Button type="button" size="compact" onClick={() => open({ kind: "team" })}>
          <Plus className="size-4" aria-hidden="true" />
          {t("teams.newTeam")}
        </Button>
      </div>
      {form?.kind === "team" && (
        <TeamForm busy={busy} errorMessage={actionErrorMessage} onSubmit={onCreateTeam} onCancel={close} />
      )}
      {teams.length === 0 ? (
        <div className="grid gap-1 py-16 text-center">
          <h2 className="text-xl font-bold">{t("home.empty.title")}</h2>
          <p className="text-muted">{t("home.empty.body")}</p>
        </div>
      ) : (
        <ul className="grid gap-4">
          {teams.map((team) => (
            <li key={team.id} className="grid gap-3 rounded-2xl border border-line bg-raised p-5">
              <div className="flex items-center justify-between gap-3">
                <h2 className="text-lg font-semibold">{team.name}</h2>
                <Button
                  type="button"
                  variant="ghost"
                  size="compact"
                  onClick={() => open({ kind: "agent", teamId: team.id })}
                >
                  <Plus className="size-4" aria-hidden="true" />
                  {t("teams.newAgent")}
                </Button>
              </div>
              {form?.kind === "agent" && form.teamId === team.id && (
                <AgentForm
                  team={team}
                  busy={busy}
                  errorMessage={actionErrorMessage}
                  onSubmit={onCreateAgent}
                  onCancel={close}
                />
              )}
              {team.agents.length === 0 ? (
                <p className="text-sm text-muted">{t("teams.noAgents")}</p>
              ) : (
                <ul className="grid gap-2">
                  {team.agents.map((agent) => (
                    <li key={agent.id} className="flex items-center gap-3">
                      <Bot className="size-5 shrink-0 text-muted" aria-hidden="true" />
                      <span className="font-medium">{agent.name}</span>
                      <span className="text-sm text-muted">{agent.role}</span>
                    </li>
                  ))}
                </ul>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
