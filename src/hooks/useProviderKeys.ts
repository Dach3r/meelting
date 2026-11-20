import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

export type ProviderKeyStatus = {
  provider: string;
  name: string;
  last_four: string | null;
  offline: boolean;
};

export type Verification =
  | { status: "verified" }
  | { status: "rejected"; reason: string }
  | { status: "offline" }
  | { status: "provider_busy" };

export type KeyAttempt =
  { verification: Verification; last_four: string | null } | { error: string };

function errorCode(error: unknown) {
  return typeof error === "string" ? error : "unknown";
}

export function useProviderKeys() {
  const [statuses, setStatuses] = useState<ProviderKeyStatus[] | null>(null);
  const [checks, setChecks] = useState<Record<string, Verification>>({});
  const [errorKey, setErrorKey] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      setStatuses(await invoke<ProviderKeyStatus[]>("list_provider_keys"));
      setErrorKey(null);
    } catch (error) {
      setErrorKey(errorCode(error));
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  async function attempt(
    provider: string,
    command: "save_provider_key" | "check_provider_key",
    args: Record<string, string>,
  ): Promise<KeyAttempt> {
    try {
      const result = await invoke<{
        verification: Verification;
        last_four: string | null;
      }>(command, { provider, ...args });
      if (
        command === "check_provider_key" ||
        result.verification.status === "verified"
      ) {
        setChecks((current) => ({
          ...current,
          [provider]: result.verification,
        }));
      }
      return result;
    } catch (error) {
      return { error: errorCode(error) };
    } finally {
      await reload();
    }
  }

  async function remove(provider: string) {
    try {
      setStatuses(
        await invoke<ProviderKeyStatus[]>("delete_provider_key", { provider }),
      );
      setChecks(({ [provider]: _removed, ...rest }) => rest);
      setErrorKey(null);
    } catch (error) {
      setErrorKey(errorCode(error));
    }
  }

  return {
    statuses,
    checks,
    errorKey,
    save: (provider: string, key: string) =>
      attempt(provider, "save_provider_key", { key }),
    check: (provider: string) => attempt(provider, "check_provider_key", {}),
    remove,
    retry: reload,
  };
}
