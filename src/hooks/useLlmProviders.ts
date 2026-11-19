import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export type LlmProvider = {
  key: string;
  name: string;
  location: "cloud" | "local";
  models: string[];
};

export function useLlmProviders() {
  const [providers, setProviders] = useState<LlmProvider[]>([]);

  useEffect(() => {
    let current = true;
    invoke<LlmProvider[]>("list_llm_providers")
      .then((loaded) => {
        if (current) setProviders(loaded);
      })
      .catch(() => {});
    return () => {
      current = false;
    };
  }, []);

  return providers;
}
