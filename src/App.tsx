import { useState } from "react";
import { NewWorkspacePage } from "./components/organisms/NewWorkspacePage";
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
