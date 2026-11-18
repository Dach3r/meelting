import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export type LocalProviderModels = { key: string; running: boolean; models: string[] };

export function useLocalModels(active: boolean) {
  const [detected, setDetected] = useState<LocalProviderModels[] | null>(null);

  useEffect(() => {
    if (!active) return;
    let current = true;
    setDetected(null);
    invoke<LocalProviderModels[]>("detect_local_models")
      .then((loaded) => {
        if (current) setDetected(loaded);
      })
      .catch(() => {
        if (current) setDetected([]);
      });
    return () => {
      current = false;
    };
  }, [active]);

  return detected;
}
