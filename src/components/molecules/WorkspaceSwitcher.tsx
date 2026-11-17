import { Check, ChevronsUpDown, Plus } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { Button } from "../atoms/Button";
import type { Workspace } from "../../hooks/useSession";
import { FormField } from "./FormField";

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
  const [open, setOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const current = workspaces.find((workspace) => workspace.id === currentId);

  async function switchTo(id: string) {
    if (await onSwitch(id)) setOpen(false);
  }

  async function create(event: FormEvent) {
    event.preventDefault();
    if (await onCreate(name)) {
      setName("");
      setCreating(false);
      setOpen(false);
    }
  }

  return (
    <div className="relative">
      <button
        type="button"
        aria-haspopup="true"
        aria-expanded={open}
        aria-label={t("home.workspace.switch")}
        onClick={() => setOpen(!open)}
        className="flex cursor-pointer items-center gap-3 rounded-full bg-surface py-2 pr-4 pl-2 text-left hover:bg-raised focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
      >
        <span className="grid size-11 shrink-0 place-items-center rounded-full bg-ink text-base font-bold text-surface">
          {current?.name.charAt(0).toUpperCase()}
        </span>
        <span className="grid min-w-0">
          <span className="truncate text-sm font-semibold">{current?.name}</span>
          <span className="text-xs text-muted">{t("home.workspace.label")}</span>
        </span>
        <ChevronsUpDown className="size-4 shrink-0 text-muted" aria-hidden="true" />
      </button>
      {open && (
        <div className="absolute top-full left-0 z-20 mt-2 w-72 max-w-[calc(100vw-1.5rem)] rounded-2xl bg-surface p-2 shadow-lg shadow-black/50">
          {workspaces.map((workspace) =>
            workspace.id === currentId ? (
              <p key={workspace.id} aria-current="true" className="flex items-center gap-3 rounded-xl p-2">
                <Initial name={workspace.name} />
                <span className="min-w-0 flex-1 truncate text-sm font-semibold">{workspace.name}</span>
                <Check className="size-4 shrink-0 text-accent" aria-hidden="true" />
              </p>
            ) : (
              <button
                key={workspace.id}
                type="button"
                disabled={busy}
                onClick={() => switchTo(workspace.id)}
                className="flex w-full cursor-pointer items-center gap-3 rounded-xl p-2 text-left hover:bg-primary focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ink disabled:cursor-not-allowed disabled:opacity-60"
              >
                <Initial name={workspace.name} />
                <span className="min-w-0 flex-1 truncate text-sm font-semibold">{workspace.name}</span>
              </button>
            ),
          )}
          {creating ? (
            <form onSubmit={create} className="grid gap-3 p-2">
              <FormField
                label={t("home.workspace.nameLabel")}
                value={name}
                onChange={(event) => setName(event.target.value)}
                maxLength={80}
                autoFocus
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
        </div>
      )}
    </div>
  );
}
