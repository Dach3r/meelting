import { FolderGit2, Plus, Trash2 } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { translateWith } from "../../i18n/translate";
import type { Repository } from "../../hooks/useRepositories";
import { Button } from "../atoms/Button";
import { Alert } from "./Alert";

type Props = {
  repositories: Repository[];
  busy: boolean;
  errorMessage: string | null;
  onRegister: () => void;
  onUnregister: (repositoryId: string) => void;
};

export function RepositoryList({
  repositories,
  busy,
  errorMessage,
  onRegister,
  onUnregister,
}: Props) {
  const { t, locale } = useI18n();
  return (
    <div className="grid gap-5 p-6 sm:px-10 sm:pb-8">
      <p className="text-sm text-muted">{t("tasks.repositories.hint")}</p>
      {repositories.length === 0 ? (
        <p className="rounded-2xl border border-line px-4 py-6 text-center text-sm text-muted">
          {t("tasks.repositories.empty")}
        </p>
      ) : (
        <ul
          aria-label={t("tasks.repositories.title")}
          className="grid overflow-hidden rounded-2xl border border-line"
        >
          {repositories.map((repository) => (
            <li
              key={repository.id}
              className="flex min-w-0 items-center gap-3 border-b border-line/60 px-4 py-3 last:border-b-0"
            >
              <FolderGit2 aria-hidden className="size-5 shrink-0 text-muted" />
              <div className="grid min-w-0 flex-1">
                <span className="truncate font-semibold text-ink">
                  {repository.name}
                </span>
                <span
                  className="truncate text-xs text-muted"
                  title={repository.path}
                >
                  {repository.path}
                </span>
              </div>
              <button
                type="button"
                aria-label={translateWith(locale, "tasks.repositories.remove", {
                  name: repository.name,
                })}
                title={translateWith(locale, "tasks.repositories.remove", {
                  name: repository.name,
                })}
                disabled={busy}
                onClick={() => onUnregister(repository.id)}
                className="grid size-9 shrink-0 cursor-pointer place-items-center rounded-full text-muted hover:bg-hi hover:text-error focus-visible:outline-2 focus-visible:outline-ink disabled:cursor-not-allowed disabled:opacity-60"
              >
                <Trash2 aria-hidden className="size-4" />
              </button>
            </li>
          ))}
        </ul>
      )}
      {errorMessage && <Alert message={errorMessage} />}
      <div className="flex justify-end">
        <Button type="button" size="pill" disabled={busy} onClick={onRegister}>
          <Plus aria-hidden className="size-4" />
          {t("tasks.repositories.add")}
        </Button>
      </div>
    </div>
  );
}
