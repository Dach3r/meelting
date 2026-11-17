import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { Locale } from "../i18n/translate";

export type AgentTemplate = {
  key: string;
  name: string;
  role: string;
  description: string;
  instructions: string;
};

export function useAgentTemplates(locale: Locale) {
  const [templates, setTemplates] = useState<AgentTemplate[]>([]);
  const [errorKey, setErrorKey] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    invoke<AgentTemplate[]>("list_agent_templates")
      .then((loaded) => {
        if (!current) return;
        setTemplates(loaded);
        setErrorKey(null);
      })
      .catch((error) => {
        if (current) setErrorKey(String(error));
      });
    return () => {
      current = false;
    };
  }, [locale]);

  return { templates, errorKey };
}
