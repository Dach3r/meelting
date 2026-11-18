import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export function useAgentRoles() {
  const [roles, setRoles] = useState<string[]>([]);

  useEffect(() => {
    let current = true;
    invoke<string[]>("list_agent_roles")
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
