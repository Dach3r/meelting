import { Building2, Check, ChevronsUpDown, Plus } from "lucide-react";
import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { Button } from "../atoms/Button";
import type { Workspace } from "../../hooks/useSession";
import { FormField } from "./FormField";
import { IslandMenu } from "./IslandMenu";

export type WorkspaceSwitcherProps = {
  workspaces: Workspace[];
  currentId: string;
  busy: boolean;
  errorMessage: string | null;
  onSwitch: (workspaceId: string) => Promise<boolean>;
  onCreate: (name: string) => Promise<boolean>;
};

function Initial({ name }: { name: string }) {
  return (
    <span className="grid size-8 shrink-0 place-items-center rounded-lg bg-ink text-xs font-bold text-surface">
      {name.charAt(0).toUpperCase()}
    </span>
  );
}

export function WorkspaceSwitcher({
  workspaces,
  currentId,
  busy,
  errorMessage,
  onSwitch,
  onCreate,
}: WorkspaceSwitcherProps) {
  const { t } = useI18n();
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const current = workspaces.find((workspace) => workspace.id === currentId);

  return (
    <IslandMenu
      label={t("home.workspace.switch")}
      trigger={
        <>
          <span className="grid size-10 shrink-0 place-items-center rounded-full bg-ink text-base font-bold text-surface">
            {current?.name.charAt(0).toUpperCase()}
          </span>
          <span className="min-w-0 flex-1">
            <span className="block max-w-56 truncate text-base leading-tight font-semibold">{current?.name}</span>
            <span className="block max-w-56 truncate text-sm text-muted">{t("home.workspace.label")}</span>
          </span>
          <ChevronsUpDown className="size-5 shrink-0 text-muted" aria-hidden="true" />
        </>
      }
    >
      {(close) => (
        <>
          {workspaces.map((workspace) =>
            workspace.id === currentId ? (
              <p key={workspace.id} aria-current="true" className="flex w-full items-center gap-3 rounded-xl p-2 text-left">
                <Initial name={workspace.name} />
                <span className="min-w-0 flex-1 truncate text-sm font-semibold">{workspace.name}</span>
                <Check className="size-4 shrink-0 text-accent" aria-hidden="true" />
              </p>
            ) : (
              <button
                key={workspace.id}
                type="button"
                disabled={busy}
                onClick={async () => {
                  if (await onSwitch(workspace.id)) close();
                }}
                className="flex w-full cursor-pointer items-center gap-3 rounded-xl p-2 text-left hover:bg-primary focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ink disabled:cursor-not-allowed disabled:opacity-60"
              >
                <Initial name={workspace.name} />
                <span className="min-w-0 flex-1 truncate text-sm font-semibold">{workspace.name}</span>
              </button>
            ),
          )}
          {creating ? (
            <form
              onSubmit={async (event) => {
                event.preventDefault();
                if (await onCreate(name)) {
                  setName("");
                  setCreating(false);
                  close();
                }
              }}
              className="grid gap-3 p-2"
            >
              <FormField
                label={t("home.workspace.nameLabel")}
                icon={Building2}
                value={name}
                onChange={(event) => setName(event.target.value)}
                maxLength={80}
                autoFocus
                autoComplete="organization"
                error={errorMessage}
              />
              {errorMessage && <p className="text-sm text-error">{errorMessage}</p>}
              <Button type="submit" size="compact" disabled={busy || name.trim() === ""}>
                {t("home.workspace.submit")}
              </Button>
            </form>
          ) : (
            <button
              type="button"
              onClick={() => setCreating(true)}
              className="flex w-full cursor-pointer items-center gap-3 rounded-xl px-3 py-3.5 text-sm text-muted hover:bg-primary hover:text-ink focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ink"
            >
              <Plus className="size-4" aria-hidden="true" />
              {t("home.workspace.create")}
            </button>
          )}
        </>
      )}
    </IslandMenu>
  );
}
