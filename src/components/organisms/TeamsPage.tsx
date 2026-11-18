import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Agent, AgentDetail, AgentEdit, Team } from "../../hooks/useTeams";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
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
  onHireAgent: (teamId: string) => void;
  onLoadAgent: (agentId: string) => Promise<AgentDetail | null>;
  onUpdateAgent: (agentId: string, edit: AgentEdit) => Promise<boolean>;
  onDeleteAgent: (agentId: string) => Promise<boolean>;
  onClearActionError: () => void;
};

export function TeamsPage({
  teams,
  errorMessage,
  actionErrorMessage,
  busy,
  onCreateTeam,
  onHireAgent,
  onLoadAgent,
  onUpdateAgent,
  onDeleteAgent,
  onClearActionError,
}: Props) {
  const { t } = useI18n();
  const [creatingTeam, setCreatingTeam] = useState(false);
  const [editing, setEditing] = useState<AgentDetail | null>(null);
  const [deleting, setDeleting] = useState<Agent | null>(null);
  const [zoom, setZoom] = useState(1);

  function openTeamForm() {
    onClearActionError();
    setCreatingTeam(true);
  }

  function close() {
    onClearActionError();
    setCreatingTeam(false);
    setEditing(null);
    setDeleting(null);
  }

  async function editAgent(agentId: string) {
    const agent = await onLoadAgent(agentId);
    if (agent) setEditing(agent);
  }

  function askToDelete(agentId: string) {
    onClearActionError();
    setDeleting(teams.flatMap((team) => team.agents).find((agent) => agent.id === agentId) ?? null);
  }

  async function confirmDelete(agent: Agent) {
    if (await onDeleteAgent(agent.id)) setDeleting(null);
  }

  if (errorMessage) {
    return (
      <div className="w-full max-w-4xl">
        <Alert message={errorMessage} />
      </div>
    );
  }

  return (
    <div className="flex min-h-0 w-full flex-1 flex-col gap-6">
      {creatingTeam && (
        <Modal title={t("teams.newTeam")} onClose={close}>
          <TeamForm busy={busy} errorMessage={actionErrorMessage} onSubmit={onCreateTeam} onCancel={close} />
        </Modal>
      )}
      {editing && (
        <Modal title={`${t("teams.agent.edit")} · ${editing.name}`} onClose={close} bare>
          <AgentForm
            team={teams.find((team) => team.id === editing.team_id) ?? teams[0]}
            edit={{ agent: editing, teams }}
            busy={busy}
            errorMessage={actionErrorMessage}
            onSubmit={({ team_id, name, role, description, instructions }) =>
              onUpdateAgent(editing.id, { team_id, name, role, description, instructions })
            }
            onCancel={close}
          />
        </Modal>
      )}
      {deleting && (
        <Modal title={`${t("teams.agent.deleteTitle")} · ${deleting.name}`} onClose={close}>
          <div className="grid gap-5 p-6">
            <p className="text-muted">{t("teams.agent.deleteBody")}</p>
            {actionErrorMessage && <Alert message={actionErrorMessage} />}
            <div className="flex justify-end gap-3">
              <Button type="button" variant="ghost" size="compact" onClick={close}>
                {t("teams.cancel")}
              </Button>
              <Button type="button" size="compact" disabled={busy} onClick={() => confirmDelete(deleting)}>
                {t("teams.agent.delete")}
              </Button>
            </div>
          </div>
        </Modal>
      )}
      <div className="board-dots relative min-h-112 flex-1 overflow-hidden rounded-2xl border border-line bg-surface">
        <OrgChart teams={teams} zoom={zoom} onNewTeam={openTeamForm} onNewAgent={onHireAgent} onEditAgent={editAgent}
          onDeleteAgent={askToDelete}
        />
        {teams.length > 0 && (
          <div className="absolute bottom-4 left-4">
            <ZoomControls zoom={zoom} onChange={setZoom} />
          </div>
        )}
      </div>
      {teams.length === 0 && (
        <div className="grid gap-1 text-center">
          <h2 className="text-xl font-bold">{t("home.empty.title")}</h2>
          <p className="text-muted">{t("home.empty.body")}</p>
        </div>
      )}
    </div>
  );
}
