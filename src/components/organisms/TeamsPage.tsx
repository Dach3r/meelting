import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Team } from "../../hooks/useTeams";
import { Alert } from "../molecules/Alert";
import { Modal } from "../molecules/Modal";
import { TeamForm } from "../molecules/TeamForm";
import { ZoomControls } from "../molecules/ZoomControls";
import { OrgChart } from "./OrgChart";

type Props = {
  teams: Team[];
  errorMessage: string | null;
  actionErrorMessage: string | null;
  busy: boolean;
  onCreateTeam: (name: string) => Promise<boolean>;
  onHireAgent: (teamId: string) => void;
  onClearActionError: () => void;
};

export function TeamsPage({
  teams,
  errorMessage,
  actionErrorMessage,
  busy,
  onCreateTeam,
  onHireAgent,
  onClearActionError,
}: Props) {
  const { t } = useI18n();
  const [creatingTeam, setCreatingTeam] = useState(false);
  const [zoom, setZoom] = useState(1);

  function openTeamForm() {
    onClearActionError();
    setCreatingTeam(true);
  }

  function close() {
    onClearActionError();
    setCreatingTeam(false);
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
      {creatingTeam && (
        <Modal title={t("teams.newTeam")} onClose={close}>
          <TeamForm busy={busy} errorMessage={actionErrorMessage} onSubmit={onCreateTeam} onCancel={close} />
        </Modal>
      )}
      <OrgChart
        teams={teams}
        zoom={zoom}
        onNewTeam={openTeamForm}
        onNewAgent={onHireAgent}
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
