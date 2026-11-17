import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export function useWorktreeRun() {
  const [output, setOutput] = useState("");
  const [status, setStatus] = useState("");

  useEffect(() => {
    const outputListener = listen<string>("terminal-output", (event) =>
      setOutput((current) => current + event.payload),
    );
    const exitListener = listen<number>("terminal-exit", (event) =>
      setStatus(`exit ${event.payload}`),
    );
    return () => {
      outputListener.then((unlisten) => unlisten());
      exitListener.then((unlisten) => unlisten());
    };
  }, []);

  async function run(repoPath: string, command: string) {
    setOutput("");
    setStatus("running");
    try {
      const worktree = await invoke<string>("run_in_worktree", {
        repoPath,
        command,
      });
      setStatus(`running in ${worktree}`);
    } catch (error) {
      setStatus(String(error));
    }
  }

  return { output, status, run };
}
