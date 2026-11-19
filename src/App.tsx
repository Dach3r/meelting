import { useEffect, useState } from "react";
import { NewWorkspacePage } from "./components/organisms/NewWorkspacePage";
import { Workspace } from "./components/templates/Workspace";
import { useSession } from "./hooks/useSession";
import { useTeams } from "./hooks/useTeams";
import { I18nProvider } from "./i18n/I18nProvider";
import { translateError, type Locale } from "./i18n/translate";

function App() {
  const {
    session,
    loaded,
    busy,
    errorKey,
    clearError,
    completeFirstLaunch,
    createWorkspace,
    switchWorkspace,
    changeLocale,
    changeTheme,
    changeUiScale,
    changeLoadingEnabled,
  } = useSession();
  const {
    teams,
    errorKey: teamsErrorKey,
    actionErrorKey: teamsActionErrorKey,
    busy: teamsBusy,
    createTeam,
    createAgent,
    updateAgent,
    deleteAgent,
    moveNode,
    resetPositions,
    loadAgent,
    clearActionError: clearTeamsActionError,
  } = useTeams(session?.current_workspace_id ?? null);
  const [draftLocale, setDraftLocale] = useState<Locale>("en");
  const [creating, setCreating] = useState(false);

  const theme = session?.theme ?? "system";
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  const uiScale = session?.ui_scale ?? 100;
  useEffect(() => {
    document.documentElement.style.fontSize = `${uiScale}%`;
  }, [uiScale]);

  if (!loaded) return null;

  const locale = session?.locale ?? draftLocale;
  const errorMessage = errorKey ? translateError(locale, errorKey) : null;
  const teamsErrorMessage = teamsErrorKey ? translateError(locale, teamsErrorKey) : null;
  const teamsActionErrorMessage = teamsActionErrorKey ? translateError(locale, teamsActionErrorKey) : null;

  return (
    <I18nProvider locale={locale}>
      {!session ? (
        <NewWorkspacePage
          busy={busy}
          errorMessage={errorMessage}
          language={{ locale: draftLocale, onChange: setDraftLocale }}
          onSubmit={(name) => completeFirstLaunch(draftLocale, name)}
        />
      ) : creating ? (
        <NewWorkspacePage
          busy={busy}
          errorMessage={errorMessage}
          onSubmit={async (name) => {
            if (await createWorkspace(name)) setCreating(false);
          }}
          onCancel={() => {
            clearError();
            setCreating(false);
          }}
        />
      ) : (
        <Workspace
          workspaces={session.workspaces}
          teams={teams}
          teamsErrorMessage={teamsErrorMessage}
          teamsActionErrorMessage={teamsActionErrorMessage}
          teamsBusy={teamsBusy}
          onCreateTeam={createTeam}
          onCreateAgent={createAgent}
          onLoadAgent={loadAgent}
          onUpdateAgent={updateAgent}
          onDeleteAgent={deleteAgent}
          onMoveNode={moveNode}
          onResetPositions={resetPositions}
          onClearTeamsActionError={clearTeamsActionError}
          locale={session.locale}
          theme={session.theme}
          uiScale={session.ui_scale}
          loadingEnabled={session.loading_enabled}
          onChangeLocale={changeLocale}
          onChangeTheme={changeTheme}
          onChangeUiScale={changeUiScale}
          onChangeLoadingEnabled={changeLoadingEnabled}
          currentId={session.current_workspace_id}
          busy={busy}
          errorMessage={errorMessage}
          onDismissError={clearError}
          onSwitch={switchWorkspace}
          onNewWorkspace={() => {
            clearError();
            setCreating(true);
          }}
        />
      )}
    </I18nProvider>
  );
}

export default App;
