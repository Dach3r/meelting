import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";

export function useDataFolder() {
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function open() {
    setBusy(true);
    setErrorKey(null);
    try {
      await invoke("open_data_folder");
    } catch (error) {
      setErrorKey(String(error));
    } finally {
      setBusy(false);
    }
  }

  return { errorKey, busy, open };
}
