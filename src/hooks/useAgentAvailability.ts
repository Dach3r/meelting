import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

export type Availability = "available" | "needs_setup" | "unavailable" | "needs_connection";

export type AgentAvailability = { agent_id: string; provider: string; availability: Availability; reason: string | null };

export function useAgentAvailability(roomId: string, finishedRuns: number) {
  const [availability, setAvailability] = useState<AgentAvailability[] | null>(null);
  const [checking, setChecking] = useState(false);
  const [requested, setRequested] = useState(0);

  useEffect(() => {
    let current = true;
    setChecking(true);
    invoke<AgentAvailability[]>("check_agent_availability", { roomId })
      .then((found) => {
        if (current) setAvailability(found);
      })
      .catch(() => {
        if (current) setAvailability(null);
      })
      .finally(() => {
        if (current) setChecking(false);
      });
    return () => {
      current = false;
    };
  }, [roomId, finishedRuns, requested]);

  const recheck = useCallback(() => setRequested((count) => count + 1), []);

  return { availability, checking, recheck };
}
