import { GitCompare, RefreshCw } from "lucide-react";
import type { ReactNode } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { translateError } from "../../i18n/translate";
import {
  rowKey,
  useEnvironmentReview,
  type EnvironmentReview,
} from "../../hooks/useEnvironmentReview";
import type { Task } from "../../hooks/useTasks";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { ChangesList } from "../molecules/ChangesList";
import { DiffViewer } from "../molecules/DiffViewer";
import { DriftNotice } from "../molecules/DriftNotice";

export function TaskChanges({ task }: { task: Task }) {
  const { t, locale } = useI18n();
  const worktree = task.worktree;
  const usable = worktree?.status === "ready" && !worktree.trashed_at;
  const changes = useEnvironmentReview(task.id, usable);
  const loading = changes.phase === "loading";

  const refresh = (label: string) => (
    <Button
      type="button"
      variant="ghost"
      size="pill"
      className="border-line"
      disabled={loading}
      onClick={changes.refresh}
    >
      <RefreshCw aria-hidden className="size-4" />
      {label}
    </Button>
  );

  return (
    <section
      aria-label={t("review.title")}
      aria-busy={usable && loading}
      className="grid gap-4 rounded-2xl border border-line bg-shell p-6"
    >
      <header className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="flex items-center gap-2 font-semibold text-ink">
          <GitCompare aria-hidden className="size-5 text-muted" />
          {t("review.title")}
        </h2>
        {usable &&
          refresh(loading ? t("review.refreshing") : t("review.refresh"))}
      </header>
      {!usable && (
        <p className="text-sm text-muted">{t("review.unavailable")}</p>
      )}
      {usable && loading && (
        <p role="status" className="text-sm text-muted">
          {t("review.loading")}
        </p>
      )}
      {usable && changes.phase === "error" && changes.errorKey && (
        <Alert
          message={translateError(locale, changes.errorKey)}
          action={refresh(t("review.retry"))}
        />
      )}
      {usable && changes.phase === "stale" && (
        <Alert message={translateError(locale, "review_stale")} />
      )}
      {usable && changes.review && changes.groups && (
        <>
          <ReviewHeader review={changes.review} />
          <DriftNotice state={changes.review.drift.state} />
          {changes.review.operation_in_progress && (
            <p role="status" className="text-sm text-ink">
              {t("review.operationNotice")}
            </p>
          )}
          {!changes.review.complete && (
            <p role="note" className="text-sm text-muted">
              {t("review.partial")}
            </p>
          )}
          {changes.review.entries.length === 0 ? (
            <p className="text-sm text-muted">{t("review.empty")}</p>
          ) : (
            <div className="grid gap-5 md:grid-cols-2">
              <ChangesList
                groups={changes.groups}
                selectedKey={changes.selected ? rowKey(changes.selected) : null}
                folders={changes.folders}
                expanded={changes.expanded}
                onSelect={changes.selectRow}
                onToggleFolder={changes.toggleFolder}
              />
              <DiffViewer row={changes.selected} state={changes.diff} />
            </div>
          )}
          <p className="text-xs text-muted">{t("review.snapshotNote")}</p>
        </>
      )}
    </section>
  );
}

function ReviewHeader({ review }: { review: EnvironmentReview }) {
  const { t } = useI18n();
  return (
    <dl className="grid gap-4 sm:grid-cols-2">
      <Detail label={t("review.header.branch")}>
        <code className="text-sm break-all">{review.branch}</code>
      </Detail>
      <Detail label={t("review.header.base")}>
        <code className="text-sm">
          {review.drift.base_ref} @ {review.base_commit_sha.slice(0, 12)}
        </code>
      </Detail>
      <Detail label={t("review.header.head")}>
        <code className="text-sm">
          {review.head_commit_sha?.slice(0, 12) ?? "—"}
        </code>
      </Detail>
      <Detail label={t("review.header.ahead")}>{review.commits_ahead}</Detail>
      <Detail label={t("review.header.drift")}>
        {t(`review.driftState.${review.drift.state}`)}
      </Detail>
      <Detail label={t("review.header.operation")}>
        {review.operation_in_progress
          ? t("review.header.operationRunning")
          : t("review.header.operationNone")}
      </Detail>
    </dl>
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
