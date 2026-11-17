import { Plus } from "lucide-react";
import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { AgentDraft, Team } from "../../hooks/useTeams";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { TeamForm } from "../molecules/TeamForm";
import { ZoomControls } from "../molecules/ZoomControls";
import { AgentForm } from "./AgentForm";
import { OrgChart } from "./OrgChart";

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
  const [zoom, setZoom] = useState(1);
  const agentTeam = form?.kind === "agent" ? teams.find((team) => team.id === form.teamId) : undefined;

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
      <div className="w-full max-w-4xl">
        <Alert message={errorMessage} />
      </div>
    );
  }

  return (
    <div className="grid w-full content-start gap-6">
      <div className="flex flex-wrap items-center justify-end gap-3">
        {teams.length > 0 && <ZoomControls zoom={zoom} onChange={setZoom} />}
        <Button type="button" size="compact" onClick={() => open({ kind: "team" })}>
          <Plus className="size-4" aria-hidden="true" />
          {t("teams.newTeam")}
        </Button>
      </div>
      {form?.kind === "team" && (
        <div className="w-full max-w-4xl">
          <TeamForm busy={busy} errorMessage={actionErrorMessage} onSubmit={onCreateTeam} onCancel={close} />
        </div>
      )}
      {agentTeam && (
        <div className="w-full max-w-4xl">
          <AgentForm
            key={agentTeam.id}
            team={agentTeam}
            busy={busy}
            errorMessage={actionErrorMessage}
            onSubmit={onCreateAgent}
            onCancel={close}
          />
        </div>
      )}
      {teams.length === 0 ? (
        <div className="grid gap-1 py-16 text-center">
          <h2 className="text-xl font-bold">{t("home.empty.title")}</h2>
          <p className="text-muted">{t("home.empty.body")}</p>
        </div>
      ) : (
        <OrgChart teams={teams} zoom={zoom} onNewAgent={(teamId) => open({ kind: "agent", teamId })} />
      )}
    </div>
  );
}
