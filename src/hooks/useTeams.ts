import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";

export type Agent = {
  id: string;
  team_id: string;
  name: string;
  role: string;
  x: number;
  y: number;
};

export type Team = {
  id: string;
  name: string;
  x: number;
  y: number;
  agents: Agent[];
};

type Position = { node_id: string; x: number; y: number };

const saveDelayMs = 500;

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

export type AgentEdit = Pick<
  AgentDraft,
  | "team_id"
  | "name"
  | "role"
  | "description"
  | "instructions"
  | "provider"
  | "model"
>;

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

  const run = useCallback(
    async (command: string, args: Record<string, unknown>) => {
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
    },
    [],
  );

  const pendingPositions = useRef(new Map<string, Position>());
  const saveTimer = useRef<ReturnType<typeof setTimeout>>(undefined);

  const savePositions = useCallback(() => {
    clearTimeout(saveTimer.current);
    const positions = [...pendingPositions.current.values()];
    pendingPositions.current.clear();
    if (positions.length > 0)
      invoke("save_canvas_positions", { positions }).catch(() => {});
  }, []);

  useEffect(() => savePositions, [savePositions]);

  const resetPositions = () => {
    pendingPositions.current.clear();
    clearTimeout(saveTimer.current);
    return run("reset_canvas_positions", {});
  };

  const moveNode = (nodeId: string, x: number, y: number) => {
    setTeams((current) =>
      current.map((team) =>
        team.id === nodeId
          ? { ...team, x, y }
          : {
              ...team,
              agents: team.agents.map((agent) =>
                agent.id === nodeId ? { ...agent, x, y } : agent,
              ),
            },
      ),
    );
    pendingPositions.current.set(nodeId, { node_id: nodeId, x, y });
    clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(savePositions, saveDelayMs);
  };

  const createTeam = (name: string) => run("create_team", { name });
  const createAgent = (draft: AgentDraft) => run("create_agent", { draft });
  const updateAgent = (agentId: string, edit: AgentEdit) =>
    run("update_agent", { agentId, edit });
  const deleteAgent = (agentId: string) => run("delete_agent", { agentId });
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

  return {
    teams,
    errorKey,
    actionErrorKey,
    busy,
    createTeam,
    createAgent,
    updateAgent,
    deleteAgent,
    moveNode,
    resetPositions,
    loadAgent,
    clearActionError,
  };
}
