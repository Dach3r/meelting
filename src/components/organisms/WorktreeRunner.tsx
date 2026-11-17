import { useState } from "react";
import { useWorktreeRun } from "../../hooks/useWorktreeRun";

export function WorktreeRunner() {
  const [repoPath, setRepoPath] = useState("");
  const [command, setCommand] = useState("");
  const { output, status, run } = useWorktreeRun();

  return (
    <section>
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
      <button onClick={() => run(repoPath, command)}>Run in worktree</button>
      <p>{status}</p>
      <pre>{output}</pre>
    </section>
  );
}
