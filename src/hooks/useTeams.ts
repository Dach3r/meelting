import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

export type Agent = { id: string; team_id: string; name: string; role: string };

export type Team = { id: string; name: string; agents: Agent[] };

export type AgentDraft = {
  team_id: string;
  template_key: string | null;
  name: string;
  role: string;
  description: string;
  instructions: string;
  provider: string;
  model: string;
};

export type AgentEdit = Pick<AgentDraft, "team_id" | "name" | "role" | "description" | "instructions">;

export type AgentDetail = AgentDraft & { id: string };

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
  const updateAgent = (agentId: string, edit: AgentEdit) => run("update_agent", { agentId, edit });
  const loadAgent = async (agentId: string) => {
    setActionErrorKey(null);
    try {
      return await invoke<AgentDetail>("get_agent", { agentId });
    } catch (error) {
      setActionErrorKey(String(error));
      return null;
    }
  };
  const clearActionError = () => setActionErrorKey(null);

  return { teams, errorKey, actionErrorKey, busy, createTeam, createAgent, updateAgent, loadAgent, clearActionError };
}
