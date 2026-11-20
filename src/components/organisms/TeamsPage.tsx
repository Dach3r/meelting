import { LayoutDashboard, Trash2 } from "lucide-react";
import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { AgentDetail, AgentEdit, Team } from "../../hooks/useTeams";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { Modal } from "../molecules/Modal";
import { TeamForm } from "../molecules/TeamForm";
import { ZoomControls } from "../molecules/ZoomControls";
import { AgentEditForm } from "./AgentEditForm";
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
  onDeleteTeam: (teamId: string) => Promise<boolean>;
  onDeleteAgent: (agentId: string) => Promise<boolean>;
  onMoveNode: (nodeId: string, x: number, y: number) => void;
  onResetPositions: () => Promise<boolean>;
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
  onDeleteTeam,
  onDeleteAgent,
  onMoveNode,
  onResetPositions,
  onClearActionError,
}: Props) {
  const { t } = useI18n();
  const [creatingTeam, setCreatingTeam] = useState(false);
  const [editing, setEditing] = useState<AgentDetail | null>(null);
  const [deleting, setDeleting] = useState<{
    kind: "agent" | "team";
    id: string;
    name: string;
  } | null>(null);
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

  function askToDeleteAgent(agentId: string) {
    const agent = teams
      .flatMap((team) => team.agents)
      .find((candidate) => candidate.id === agentId);
    if (!agent) return;
    onClearActionError();
    setDeleting({ kind: "agent", id: agent.id, name: agent.name });
  }

  function askToDeleteTeam(teamId: string) {
    const team = teams.find((candidate) => candidate.id === teamId);
    if (!team) return;
    onClearActionError();
    setDeleting({ kind: "team", id: team.id, name: team.name });
  }

  async function confirmDelete(target: NonNullable<typeof deleting>) {
    const deleted =
      target.kind === "agent"
        ? await onDeleteAgent(target.id)
        : await onDeleteTeam(target.id);
    if (deleted) setDeleting(null);
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
          <TeamForm
            busy={busy}
            errorMessage={actionErrorMessage}
            onSubmit={onCreateTeam}
            onCancel={close}
          />
        </Modal>
      )}
      {editing && (
        <Modal
          title={`${t("teams.agent.edit")} · ${editing.name}`}
          onClose={close}
          bare
        >
          <AgentEditForm
            agent={editing}
            teams={teams}
            busy={busy}
            errorMessage={actionErrorMessage}
            onSubmit={(edit) => onUpdateAgent(editing.id, edit)}
            onClose={close}
          />
        </Modal>
      )}
      {deleting && (
        <Modal
          title={t(`teams.${deleting.kind}.deleteTitle`).replace(
            "{name}",
            deleting.name,
          )}
          onClose={close}
          bare
          narrow
        >
          <div className="grid gap-5 p-6">
            <div className="flex items-start gap-4">
              <span className="grid size-11 shrink-0 place-items-center rounded-2xl bg-error/10 text-error">
                <Trash2 aria-hidden className="size-5" />
              </span>
              <div className="grid gap-1.5">
                <h2 className="text-lg font-bold text-ink">
                  {t(`teams.${deleting.kind}.deleteTitle`).replace(
                    "{name}",
                    deleting.name,
                  )}
                </h2>
                <p className="text-sm text-muted">
                  {t(`teams.${deleting.kind}.deleteBody`)}
                </p>
              </div>
            </div>
            {actionErrorMessage && <Alert message={actionErrorMessage} />}
            <div className="flex justify-end gap-3">
              <Button
                type="button"
                variant="ghost"
                size="compact"
                onClick={close}
              >
                {t("teams.cancel")}
              </Button>
              <Button
                type="button"
                variant="destructive"
                size="compact"
                disabled={busy}
                onClick={() => confirmDelete(deleting)}
              >
                {t(`teams.${deleting.kind}.delete`)}
              </Button>
            </div>
          </div>
        </Modal>
      )}
      <div className="board-dots relative min-h-112 flex-1 overflow-hidden rounded-2xl border border-line bg-surface">
        <OrgChart
          teams={teams}
          zoom={zoom}
          onNewTeam={openTeamForm}
          onNewAgent={onHireAgent}
          onEditAgent={editAgent}
          onLoadAgent={onLoadAgent}
          onDeleteTeam={askToDeleteTeam}
          onDeleteAgent={askToDeleteAgent}
          onMoveNode={onMoveNode}
        />
        {teams.length > 0 && (
          <div className="absolute bottom-4 left-4 flex items-center gap-2">
            <ZoomControls zoom={zoom} onChange={setZoom} />
            <button
              type="button"
              aria-label={t("teams.resetLayout")}
              title={t("teams.resetLayout")}
              disabled={busy}
              onClick={onResetPositions}
              className="grid size-12 cursor-pointer place-items-center rounded-2xl border border-line bg-raised text-muted shadow-sm transition-colors enabled:hover:bg-primary enabled:hover:text-ink disabled:cursor-not-allowed disabled:opacity-40 focus-visible:outline-2 focus-visible:outline-ink"
            >
              <LayoutDashboard className="size-4" aria-hidden="true" />
            </button>
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
