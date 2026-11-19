import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

export type Repository = { id: string; name: string; path: string };

export type RepositoryBranches = {
  branches: string[];
  default_branch: string | null;
};

export function useRepositories() {
  const [repositories, setRepositories] = useState<Repository[] | null>(null);
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [actionErrorKey, setActionErrorKey] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let current = true;
    invoke<Repository[]>("list_repositories")
      .then((loaded) => {
        if (current) setRepositories(loaded);
      })
      .catch((error) => {
        if (current) setErrorKey(String(error));
      });
    return () => {
      current = false;
    };
  }, []);

  async function run(action: () => Promise<Repository[] | null>) {
    setBusy(true);
    setActionErrorKey(null);
    try {
      const updated = await action();
      if (updated) setRepositories(updated);
    } catch (error) {
      setActionErrorKey(String(error));
    } finally {
      setBusy(false);
    }
  }

  const registerRepository = () =>
    run(() => invoke<Repository[] | null>("register_repository"));

  const unregisterRepository = (repositoryId: string) =>
    run(() => invoke<Repository[]>("unregister_repository", { repositoryId }));

  const loadBranches = useCallback(
    (repositoryId: string) =>
      invoke<RepositoryBranches>("list_repository_branches", {
        repositoryId,
      }),
    [],
  );

  return {
    repositories,
    errorKey,
    actionErrorKey,
    busy,
    registerRepository,
    unregisterRepository,
    loadBranches,
    clearActionError: () => setActionErrorKey(null),
  };
}
