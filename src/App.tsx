import { useState } from "react";
import { FirstLaunchCard } from "./components/organisms/FirstLaunchCard";
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
    completeFirstLaunch,
    createWorkspace,
    switchWorkspace,
  } = useSession();
  const [draftLocale, setDraftLocale] = useState<Locale>("en");

  if (!loaded) return null;

  const locale = session?.locale ?? draftLocale;
  const errorMessage = errorKey ? translateError(locale, errorKey) : null;

  return (
    <I18nProvider locale={locale}>
      {session ? (
        <Workspace
          workspaces={session.workspaces}
          currentId={session.current_workspace_id}
          busy={busy}
          errorMessage={errorMessage}
          onSwitch={switchWorkspace}
          onCreate={createWorkspace}
        />
      ) : (
        <FirstLaunchCard
          locale={draftLocale}
          busy={busy}
          errorMessage={errorMessage}
          onLocaleChange={setDraftLocale}
          onSubmit={(name) => completeFirstLaunch(draftLocale, name)}
        />
      )}
    </I18nProvider>
  );
}

export default App;
