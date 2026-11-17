import { useState } from "react";
import { NewWorkspaceCard } from "./components/organisms/NewWorkspaceCard";
import { Workspace } from "./components/templates/Workspace";
import { useSession } from "./hooks/useSession";
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
  } = useSession();
  const [draftLocale, setDraftLocale] = useState<Locale>("en");
  const [creating, setCreating] = useState(false);

  if (!loaded) return null;

  const locale = session?.locale ?? draftLocale;
  const errorMessage = errorKey ? translateError(locale, errorKey) : null;

  return (
    <I18nProvider locale={locale}>
      {!session ? (
        <NewWorkspaceCard
          busy={busy}
          errorMessage={errorMessage}
          language={{ locale: draftLocale, onChange: setDraftLocale }}
          onSubmit={(name) => completeFirstLaunch(draftLocale, name)}
        />
      ) : creating ? (
        <NewWorkspaceCard
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
          currentId={session.current_workspace_id}
          busy={busy}
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
