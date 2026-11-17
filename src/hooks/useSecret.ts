import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";

export function useSecret() {
  const [status, setStatus] = useState("");

  async function save(name: string, secret: string) {
    try {
      await invoke("save_secret", { name, secret });
      const lastFour = await invoke<string | null>("secret_last_four", {
        name,
      });
      setStatus(`saved ····${lastFour}`);
    } catch (error) {
      setStatus(String(error));
    }
  }

  return { status, save };
}
