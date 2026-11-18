import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

type RoleGroup = { group: string; roles: string[] };

export function useAgentRoles() {
  const [roles, setRoles] = useState<RoleGroup[]>([]);

  useEffect(() => {
    let current = true;
    invoke<RoleGroup[]>("list_agent_roles")
      .then((loaded) => {
        if (current) setRoles(loaded);
      })
      .catch(() => {});
    return () => {
      current = false;
    };
  }, []);

  return roles;
}
