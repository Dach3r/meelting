import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

function App() {
  const [repoPath, setRepoPath] = useState("");
  const [command, setCommand] = useState("");
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

  async function run() {
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

  return (
    <main style={{ padding: 24, fontFamily: "system-ui" }}>
      <h1>Hello, world</h1>
      <input
        placeholder="repo path"
        value={repoPath}
        onChange={(event) => setRepoPath(event.target.value)}
      />
      <input
        placeholder="command"
        value={command}
        onChange={(event) => setCommand(event.target.value)}
      />
      <button onClick={run}>Run in worktree</button>
      <p>{status}</p>
      <pre>{output}</pre>
    </main>
  );
}

export default App;
