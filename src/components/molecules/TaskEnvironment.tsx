import { FolderGit2 } from "lucide-react";
import { useState, type ReactNode } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { formatDateTime, translateWith } from "../../i18n/translate";
import type { RemovalCheck, Task } from "../../hooks/useTasks";
import { Button } from "../atoms/Button";
import { Modal } from "./Modal";

type Props = {
  task: Task;
  busy: boolean;
  onPrepare: () => void;
  onDiscard: () => void;
  onCheckRemoval: () => Promise<RemovalCheck | null>;
  onRemove: () => Promise<void>;
};

export function TaskEnvironment({
  task,
  busy,
  onPrepare,
  onDiscard,
  onCheckRemoval,
  onRemove,
}: Props) {
  const { t, locale } = useI18n();
  const [removal, setRemoval] = useState<RemovalCheck | null>(null);
  const worktree = task.worktree;

  async function askRemoval() {
    setRemoval(await onCheckRemoval());
  }

  async function confirmRemoval() {
    setRemoval(null);
    await onRemove();
  }
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
      {worktree?.status === "ready" && (
        <Button
          type="button"
          variant="danger"
          size="pill"
          className="justify-self-start"
          disabled={busy}
          onClick={askRemoval}
        >
          {t("tasks.environment.remove")}
        </Button>
      )}
      {worktree && removal && (
        <RemovalDialog
          check={removal}
          path={worktree.path}
          branch={worktree.branch_name}
          busy={busy}
          onConfirm={confirmRemoval}
          onClose={() => setRemoval(null)}
        />
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

function RemovalDialog({
  check,
  path,
  branch,
  busy,
  onConfirm,
  onClose,
}: {
  check: RemovalCheck;
  path: string;
  branch: string;
  busy: boolean;
  onConfirm: () => void;
  onClose: () => void;
}) {
  const { t, locale } = useI18n();
  const blocked = check.changes.length > 0 || check.operation_in_progress;
  return (
    <Modal
      title={
        blocked
          ? t("tasks.environment.removeDialog.blockedTitle")
          : t("tasks.environment.removeDialog.title")
      }
      onClose={onClose}
    >
      <div className="grid gap-4 p-6">
        {blocked ? (
          <>
            {check.operation_in_progress && (
              <p className="text-sm text-error">
                {t("tasks.environment.removeDialog.operation")}
              </p>
            )}
            {check.changes.length > 0 && (
              <>
                <p className="text-sm text-muted">
                  {t("tasks.environment.removeDialog.blockedBody")}
                </p>
                <ul
                  aria-label={t("tasks.environment.removeDialog.blockedTitle")}
                  className="grid max-h-64 overflow-y-auto rounded-2xl border border-line"
                >
                  {check.changes.map((change) => (
                    <li
                      key={`${change.kind}:${change.path}`}
                      className="flex min-w-0 items-center gap-3 border-b border-line/60 px-4 py-2 text-sm last:border-b-0"
                    >
                      <span className="shrink-0 text-xs font-semibold text-muted">
                        {t(`tasks.environment.removeDialog.kinds.${change.kind}`)}
                      </span>
                      <code className="min-w-0 truncate" title={change.path}>
                        {change.path}
                      </code>
                    </li>
                  ))}
                </ul>
              </>
            )}
            <div className="flex justify-end">
              <Button type="button" size="compact" onClick={onClose}>
                {t("tasks.environment.removeDialog.keep")}
              </Button>
            </div>
          </>
        ) : (
          <>
            <p className="text-sm text-muted">
              {translateWith(locale, "tasks.environment.removeDialog.body", {
                path,
              })}
            </p>
            <p className="text-sm text-ink">
              {check.commits_ahead > 0
                ? translateWith(
                    locale,
                    "tasks.environment.removeDialog.branchWithCommits",
                    { branch, count: check.commits_ahead },
                  )
                : translateWith(
                    locale,
                    "tasks.environment.removeDialog.branchKept",
                    { branch },
                  )}
            </p>
            <div className="flex justify-end gap-3">
              <Button
                type="button"
                variant="ghost"
                size="compact"
                onClick={onClose}
              >
                {t("tasks.environment.removeDialog.keep")}
              </Button>
              <Button
                type="button"
                variant="destructive"
                size="compact"
                disabled={busy}
                onClick={onConfirm}
              >
                {t("tasks.environment.removeDialog.confirm")}
              </Button>
            </div>
          </>
        )}
      </div>
    </Modal>
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
