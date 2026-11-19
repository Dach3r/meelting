import { FolderGit2 } from "lucide-react";
import type { ReactNode } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { formatDateTime, translateWith } from "../../i18n/translate";
import type { Task } from "../../hooks/useTasks";
import { Button } from "../atoms/Button";

type Props = {
  task: Task;
  busy: boolean;
  onPrepare: () => void;
  onDiscard: () => void;
};

export function TaskEnvironment({ task, busy, onPrepare, onDiscard }: Props) {
  const { t, locale } = useI18n();
  const worktree = task.worktree;
  const closed = task.status === "completed" || task.status === "canceled";
  return (
    <section
      aria-label={t("tasks.environment.title")}
      className="grid gap-4 rounded-2xl border border-line bg-shell p-6"
    >
      <header className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="flex items-center gap-2 font-semibold text-ink">
          <FolderGit2 aria-hidden className="size-5 text-muted" />
          {t("tasks.environment.title")}
        </h2>
        {worktree && (
          <span
            className={`rounded-full border px-2.5 py-0.5 text-xs font-semibold ${worktree.status === "ready" ? "border-ink text-ink" : worktree.status === "failed" || worktree.status === "missing" ? "border-error text-error" : "border-line text-muted"}`}
          >
            {t(`tasks.environment.status.${worktree.status}`)}
          </span>
        )}
      </header>
      {!worktree && (
        <>
          <p className="text-sm text-muted">
            {!task.base_ref
              ? t("tasks.environment.noBase")
              : closed
                ? t("tasks.environment.closed")
                : translateWith(locale, "tasks.environment.notPrepared", {
                    branch: task.base_ref,
                  })}
          </p>
          <Button
            type="button"
            size="pill"
            className="justify-self-start"
            disabled={busy || closed || !task.base_ref}
            onClick={onPrepare}
          >
            {t("tasks.environment.prepare")}
          </Button>
        </>
      )}
      {worktree && (
        <dl className="grid gap-4 sm:grid-cols-2">
          <Detail label={t("tasks.environment.branch")}>
            <code className="text-sm break-all">{worktree.branch_name}</code>
          </Detail>
          <Detail label={t("tasks.environment.base")}>
            <code className="text-sm">
              {worktree.base_ref} @ {worktree.base_commit_sha.slice(0, 12)}
            </code>
          </Detail>
          <Detail label={t("tasks.environment.path")}>
            <code className="text-sm break-all select-text">
              {worktree.path}
            </code>
          </Detail>
          <Detail label={t("tasks.detail.created")}>
            {formatDateTime(locale, worktree.created_at)}
          </Detail>
        </dl>
      )}
      {(worktree?.status === "failed" || worktree?.status === "missing") && (
        <div className="grid gap-3">
          {worktree.error_code && (
            <p className="text-sm font-medium text-error">
              {t(`tasks.environment.errors.${worktree.error_code}`)}
            </p>
          )}
          <p className="text-sm text-muted">
            {worktree.status === "missing"
              ? t("tasks.environment.missingHelp")
              : t("tasks.environment.failedHelp")}
          </p>
          <Button
            type="button"
            variant="ghost"
            size="pill"
            className="justify-self-start border-line"
            disabled={busy}
            onClick={onDiscard}
          >
            {t("tasks.environment.discard")}
          </Button>
        </div>
      )}
    </section>
  );
}

function Detail({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid min-w-0 gap-1">
      <dt className="text-xs font-medium text-muted">{label}</dt>
      <dd className="min-w-0 text-ink">{children}</dd>
    </div>
  );
}
