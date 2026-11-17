import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

const SECRET_NAME = "spike/test";

function App() {
  const [secret, setSecret] = useState("");
  const [status, setStatus] = useState("");

  async function run(action: () => Promise<string>) {
    try {
      setStatus(await action());
    } catch (error) {
      setStatus(`error: ${error}`);
    }
  }

  return (
    <main style={{ padding: 24, fontFamily: "system-ui" }}>
      <h1>Meelting spike: keychain</h1>
      <input
        type="password"
        value={secret}
        onChange={(event) => setSecret(event.target.value)}
        placeholder="secret"
      />
      <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
        <button
          onClick={() =>
            run(async () => {
              await invoke("save_secret", { name: SECRET_NAME, secret });
              setSecret("");
              return "saved";
            })
          }
        >
          Save
        </button>
        <button
          onClick={() =>
            run(async () => {
              const lastFour = await invoke<string | null>("secret_last_four", { name: SECRET_NAME });
              return lastFour ? `stored: ****${lastFour}` : "nothing stored";
            })
          }
        >
          Read
        </button>
        <button
          onClick={() =>
            run(async () => {
              await invoke("delete_secret", { name: SECRET_NAME });
              return "deleted";
            })
          }
        >
          Delete
        </button>
      </div>
      <p>{status}</p>
    </main>
  );
}

export default App;
