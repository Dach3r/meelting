import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export function useOpenAtLogin() {
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let current = true;
    invoke<boolean>("get_open_at_login")
      .then((loaded) => {
        if (current) setEnabled(loaded);
      })
      .catch((error) => {
        if (current) setErrorKey(String(error));
      });
    return () => {
      current = false;
    };
  }, []);

  async function change(next: boolean) {
    setBusy(true);
    setErrorKey(null);
    try {
      setEnabled(
        await invoke<boolean>("change_open_at_login", { enabled: next }),
      );
    } catch (error) {
      setErrorKey(String(error));
    } finally {
      setBusy(false);
    }
  }

  return { enabled, errorKey, busy, change };
}
