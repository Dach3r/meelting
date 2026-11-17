import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { AgentDraft, Team } from "../../hooks/useTeams";
import { Alert } from "../molecules/Alert";
import { BotIcon, UsersIcon } from "../atoms/Icons";
import { Modal } from "../molecules/Modal";
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
      {teams.length > 0 && (
        <div className="flex justify-end">
          <ZoomControls zoom={zoom} onChange={setZoom} />
        </div>
      )}
      {form?.kind === "team" && (
        <Modal icon={<UsersIcon />} title={t("teams.newTeam")} subtitle={t("teams.newTeamHint")} onClose={close}>
          <TeamForm busy={busy} errorMessage={actionErrorMessage} onSubmit={onCreateTeam} onCancel={close} />
        </Modal>
      )}
      {agentTeam && (
        <Modal icon={<BotIcon />} title={t("teams.newAgent")} subtitle={agentTeam.name} onClose={close}>
          <AgentForm
            key={agentTeam.id}
            team={agentTeam}
            busy={busy}
            errorMessage={actionErrorMessage}
            onSubmit={onCreateAgent}
            onCancel={close}
          />
        </Modal>
      )}
      <OrgChart
        teams={teams}
        zoom={zoom}
        onNewTeam={() => open({ kind: "team" })}
        onNewAgent={(teamId) => open({ kind: "agent", teamId })}
      />
      {teams.length === 0 && (
        <div className="grid gap-1 text-center">
          <h2 className="text-xl font-bold">{t("home.empty.title")}</h2>
          <p className="text-muted">{t("home.empty.body")}</p>
        </div>
      )}
    </div>
  );
}
