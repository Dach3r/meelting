import { Bot } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Team } from "../../hooks/useTeams";
import { Alert } from "../molecules/Alert";

type Props = {
  teams: Team[];
  errorMessage: string | null;
};

export function TeamsPage({ teams, errorMessage }: Props) {
  const { t } = useI18n();

  if (errorMessage) {
    return (
      <div className="mx-auto w-full max-w-4xl">
        <Alert message={errorMessage} />
      </div>
    );
  }

  if (teams.length === 0) {
    return (
      <div className="mx-auto grid min-h-0 w-full max-w-4xl flex-1 place-content-center gap-1 text-center">
        <h1 className="text-2xl font-bold">{t("home.empty.title")}</h1>
        <p className="text-muted">{t("home.empty.body")}</p>
      </div>
    );
  }

  return (
    <div className="mx-auto grid w-full max-w-4xl content-start gap-6">
      <h1 className="text-2xl font-bold">{t("teams.title")}</h1>
      <ul className="grid gap-4">
        {teams.map((team) => (
          <li key={team.id} className="grid gap-3 rounded-2xl border border-line bg-raised p-5">
            <h2 className="text-lg font-semibold">{team.name}</h2>
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
    </div>
  );
}
