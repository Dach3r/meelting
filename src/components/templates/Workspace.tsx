import { useState } from "react";
import type { Theme } from "../../hooks/useSession";
import type { Locale } from "../../i18n/translate";
import type { AgentDetail, AgentDraft, AgentEdit, Team } from "../../hooks/useTeams";
import type { WorkspaceSwitcherProps } from "../molecules/WorkspaceSwitcher";
import { SwitchingOverlay } from "../molecules/SwitchingOverlay";
import { AgentCatalogPage } from "../organisms/AgentCatalogPage";
import { Rail, type View } from "../organisms/Rail";
import { RoomsPage } from "../organisms/RoomsPage";
import { SettingsPage } from "../organisms/SettingsPage";
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
  onLoadAgent: (agentId: string) => Promise<AgentDetail | null>;
  onUpdateAgent: (agentId: string, edit: AgentEdit) => Promise<boolean>;
  onDeleteAgent: (agentId: string) => Promise<boolean>;
  onMoveNode: (nodeId: string, x: number, y: number) => void;
  onResetPositions: () => Promise<boolean>;
  onClearTeamsActionError: () => void;
  locale: Locale;
  theme: Theme;
  onChangeLocale: (locale: Locale) => void;
  onChangeTheme: (theme: Theme) => void;
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
  onLoadAgent,
  onUpdateAgent,
  onDeleteAgent,
  onMoveNode,
  onResetPositions,
  onClearTeamsActionError,
  locale,
  theme,
  onChangeLocale,
  onChangeTheme,
  errorMessage,
  onDismissError,
  ...props
}: WorkspaceProps) {
  const [collapsed, setCollapsed] = useState(storedCollapsed);
  const [animated, setAnimated] = useState(false);
  const [view, setView] = useState<View>("teams");
  const [catalogTeamId, setCatalogTeamId] = useState<string | null>(null);
  const catalogTeam = teams.find((team) => team.id === catalogTeamId);
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

  function selectView(next: View) {
    setCatalogTeamId(null);
    setView(next);
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
      <Rail collapsed={collapsed} onToggle={toggle} view={view} onSelectView={selectView} />
      <div className="flex min-h-0 min-w-0 flex-col overflow-hidden rounded-3xl bg-shell">
        <Topbar {...props} onSwitch={switchWorkspace} />
        <main className="flex min-h-0 flex-1 flex-col px-8 pb-8">
          <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-3xl border border-line bg-pill">
            <div className="flex min-h-0 flex-1 flex-col overflow-y-auto p-8">
              {view === "settings" ? (
                <SettingsPage
                  locale={locale}
                  theme={theme}
                  busy={props.busy}
                  errorMessage={errorMessage}
                  onChangeLocale={onChangeLocale}
                  onChangeTheme={onChangeTheme}
                />
              ) : view === "rooms" ? (
                <RoomsPage key={props.currentId} agents={teams.flatMap((team) => team.agents)} />
              ) : catalogTeam ? (
                <AgentCatalogPage
                  team={catalogTeam}
                  busy={teamsBusy}
                  actionErrorMessage={teamsActionErrorMessage}
                  onCreateAgent={onCreateAgent}
                  onClearActionError={onClearTeamsActionError}
                  onBack={() => setCatalogTeamId(null)}
                />
              ) : (
                <TeamsPage
                  teams={teams}
                  errorMessage={teamsErrorMessage}
                  actionErrorMessage={teamsActionErrorMessage}
                  busy={teamsBusy}
                  onCreateTeam={onCreateTeam}
                  onHireAgent={setCatalogTeamId}
                  onLoadAgent={onLoadAgent}
                  onUpdateAgent={onUpdateAgent}
                  onDeleteAgent={onDeleteAgent}
                  onMoveNode={onMoveNode}
                  onResetPositions={onResetPositions}
                  onClearActionError={onClearTeamsActionError}
                />
              )}
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
