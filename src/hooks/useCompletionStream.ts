import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

type CompletionRequest = {
  baseUrl: string;
  model: string;
  prompt: string;
  secretName: string | null;
};

export function useCompletionStream() {
  const [output, setOutput] = useState("");
  const [status, setStatus] = useState("");

  useEffect(() => {
    const tokenListener = listen<string>("llm-token", (event) =>
      setOutput((current) => current + event.payload),
    );
    return () => {
      tokenListener.then((unlisten) => unlisten());
    };
  }, []);

  async function run(request: CompletionRequest) {
    setOutput("");
    setStatus("streaming");
    try {
      await invoke("stream_completion", request);
      setStatus("done");
    } catch (error) {
      setStatus(String(error));
    }
  }

  return { output, status, run };
}
