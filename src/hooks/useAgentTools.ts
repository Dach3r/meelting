import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export type AgentTools = { enabled: boolean; available: boolean };

export function useAgentTools() {
  const [tools, setTools] = useState<AgentTools | null>(null);
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let current = true;
    invoke<AgentTools>("get_agent_tools")
      .then((loaded) => {
        if (current) setTools(loaded);
      })
      .catch((error) => {
        if (current) setErrorKey(String(error));
      });
    return () => {
      current = false;
    };
  }, []);

  async function change(enabled: boolean) {
    setBusy(true);
    setErrorKey(null);
    try {
      setTools(await invoke<AgentTools>("change_agent_tools", { enabled }));
    } catch (error) {
      setErrorKey(String(error));
    } finally {
      setBusy(false);
    }
  }

  return { tools, errorKey, busy, change };
}
