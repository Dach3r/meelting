import { useState } from "react";
import type { AgentDraft, Team } from "../../hooks/useTeams";
import type { WorkspaceSwitcherProps } from "../molecules/WorkspaceSwitcher";
import { SwitchingOverlay } from "../molecules/SwitchingOverlay";
import { Rail } from "../organisms/Rail";
import { TeamsPage } from "../organisms/TeamsPage";
import { Topbar } from "../organisms/Topbar";

const storageKey = "sidebar";
const minSwitchMs = 600;

function storedCollapsed() {
  try {
    return localStorage.getItem(storageKey) === "collapsed";
  } catch {
    return false;
  }
}

type WorkspaceProps = WorkspaceSwitcherProps & {
  teams: Team[];
  teamsErrorMessage: string | null;
  teamsActionErrorMessage: string | null;
  teamsBusy: boolean;
  onCreateTeam: (name: string) => Promise<boolean>;
  onCreateAgent: (draft: AgentDraft) => Promise<boolean>;
  onClearTeamsActionError: () => void;
  errorMessage: string | null;
  onDismissError: () => void;
};

export function Workspace({
  teams,
  teamsErrorMessage,
  teamsActionErrorMessage,
  teamsBusy,
  onCreateTeam,
  onCreateAgent,
  onClearTeamsActionError,
  errorMessage,
  onDismissError,
  ...props
}: WorkspaceProps) {
  const [collapsed, setCollapsed] = useState(storedCollapsed);
  const [animated, setAnimated] = useState(false);
  const [switching, setSwitching] = useState<{ name: string; failed: boolean } | null>(null);

  async function switchWorkspace(workspaceId: string) {
    const target = props.workspaces.find((workspace) => workspace.id === workspaceId);
    setSwitching({ name: target?.name ?? "", failed: false });
    const [switched] = await Promise.all([
      props.onSwitch(workspaceId),
      new Promise((resolve) => setTimeout(resolve, minSwitchMs)),
    ]);
    setSwitching(switched ? null : { name: target?.name ?? "", failed: true });
    return switched;
  }

  function toggle() {
    const next = !collapsed;
    setAnimated(true);
    setCollapsed(next);
    try {
      localStorage.setItem(storageKey, next ? "collapsed" : "expanded");
    } catch {
      return;
    }
  }

  return (
    <div
      data-collapsed={collapsed || undefined}
      className={`group/rail relative grid h-dvh grid-cols-[19rem_minmax(0,1fr)] gap-3 overflow-hidden bg-rail p-6 data-collapsed:grid-cols-[4.5rem_minmax(0,1fr)] data-collapsed:pl-3 ${animated ? "transition-[grid-template-columns] duration-300 ease-in-out" : ""}`}
    >
      <Rail collapsed={collapsed} onToggle={toggle} />
      <div className="flex min-h-0 min-w-0 flex-col overflow-hidden rounded-3xl bg-shell">
        <Topbar {...props} onSwitch={switchWorkspace} />
        <main className="flex min-h-0 flex-1 flex-col px-8 pb-8">
          <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-3xl bg-surface">
            <div className="flex min-h-0 flex-1 flex-col overflow-y-auto p-8">
              <TeamsPage
                teams={teams}
                errorMessage={teamsErrorMessage}
                actionErrorMessage={teamsActionErrorMessage}
                busy={teamsBusy}
                onCreateTeam={onCreateTeam}
                onCreateAgent={onCreateAgent}
                onClearActionError={onClearTeamsActionError}
              />
            </div>
          </div>
        </main>
      </div>
      {switching && (
        <SwitchingOverlay
          workspaceName={switching.name}
          errorMessage={switching.failed ? errorMessage : null}
          onDismiss={() => {
            setSwitching(null);
            onDismissError();
          }}
        />
      )}
    </div>
  );
}
