import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

export type Agent = { id: string; team_id: string; name: string; role: string };

export type Team = { id: string; name: string; agents: Agent[] };

export type AgentDraft = {
  team_id: string;
  name: string;
  role: string;
  description: string;
  instructions: string;
  provider: string;
  model: string;
};

export function useTeams(workspaceId: string | null) {
  const [teams, setTeams] = useState<Team[]>([]);
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [actionErrorKey, setActionErrorKey] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!workspaceId) return;
    let current = true;
    invoke<Team[]>("list_teams")
      .then((loaded) => {
        if (!current) return;
        setTeams(loaded);
        setErrorKey(null);
      })
      .catch((error) => {
        if (current) setErrorKey(String(error));
      });
    return () => {
      current = false;
    };
  }, [workspaceId]);

  const run = useCallback(async (command: string, args: Record<string, unknown>) => {
    setBusy(true);
    setActionErrorKey(null);
    try {
      setTeams(await invoke<Team[]>(command, args));
      return true;
    } catch (error) {
      setActionErrorKey(String(error));
      return false;
    } finally {
      setBusy(false);
    }
  }, []);

  const createTeam = (name: string) => run("create_team", { name });
  const createAgent = (draft: AgentDraft) => run("create_agent", { draft });
  const clearActionError = () => setActionErrorKey(null);

  return { teams, errorKey, actionErrorKey, busy, createTeam, createAgent, clearActionError };
}
