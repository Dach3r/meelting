import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { Choice, Selection } from "./useRoom";

export type Resolution = { kind: "nobody" } | { kind: "agent"; agent_id: string } | ({ kind: "choose" } & Choice);

const previewDelayMs = 150;

export function useRecipientPreview(roomId: string, body: string, selection: Selection, mentionIds: string[]) {
  const [resolution, setResolution] = useState<Resolution | null>(null);
  const empty = body.trim().length === 0;

  useEffect(() => {
    if (empty) return;
    let current = true;
    const timer = setTimeout(() => {
      invoke<Resolution>("resolve_recipient", { roomId, body, selection, mentionIds })
        .then((resolved) => {
          if (current) setResolution(resolved);
        })
        .catch(() => {
          if (current) setResolution(null);
        });
    }, previewDelayMs);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [roomId, body, empty, selection, mentionIds]);

  return empty ? null : resolution;
}
