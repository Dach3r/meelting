import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export type Agent = { id: string; team_id: string; name: string; role: string };

export type Team = { id: string; name: string; agents: Agent[] };

export function useTeams(workspaceId: string | null) {
  const [teams, setTeams] = useState<Team[]>([]);
  const [errorKey, setErrorKey] = useState<string | null>(null);

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

  return { teams, errorKey };
}
