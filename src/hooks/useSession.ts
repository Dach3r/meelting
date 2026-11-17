import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import type { Locale } from "../i18n/translate";

export type Workspace = { id: string; name: string };

export type Session = {
  locale: Locale;
  current_workspace_id: string;
  workspaces: Workspace[];
};

export function useSession() {
  const [session, setSession] = useState<Session | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    invoke<Session | null>("load_session")
      .then(setSession)
      .catch((error) => setErrorKey(String(error)))
      .finally(() => setLoaded(true));
  }, []);

  const run = useCallback(async (command: string, args: Record<string, unknown>) => {
    setBusy(true);
    setErrorKey(null);
    try {
      setSession(await invoke<Session>(command, args));
      return true;
    } catch (error) {
      setErrorKey(String(error));
      return false;
    } finally {
      setBusy(false);
    }
  }, []);

  const completeFirstLaunch = (locale: Locale, workspaceName: string) =>
    run("complete_first_launch", { locale, workspaceName });
  const createWorkspace = (name: string) => run("create_workspace", { name });
  const switchWorkspace = (workspaceId: string) =>
    run("switch_workspace", { workspaceId });

  return {
    session,
    loaded,
    busy,
    errorKey,
    completeFirstLaunch,
    createWorkspace,
    switchWorkspace,
  };
}
