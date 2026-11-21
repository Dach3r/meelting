import { useState } from "react";
import { useEnvironments, type Environment } from "../../hooks/useEnvironments";
import { useI18n } from "../../i18n/I18nProvider";
import { translateError, translateWith } from "../../i18n/translate";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { ConfirmDialog } from "../molecules/ConfirmDialog";
import { EnvironmentDetail } from "../molecules/EnvironmentDetail";
import { EnvironmentsEmptyGuide } from "../molecules/EnvironmentsEmptyGuide";
import { TrashEmptyCalendar } from "../molecules/TrashEmptyCalendar";
import { EnvironmentList } from "../molecules/EnvironmentList";

const trashDays = 30;
const dayMs = 24 * 60 * 60 * 1000;

function daysLeft(trashedAt: string, now: number) {
  const elapsed = Math.floor((now - new Date(trashedAt).getTime()) / dayMs);
  return Math.max(0, trashDays - elapsed);
}

const dialogKeys = {
  trash: "files.trashDialog",
  purge: "trash.purgeDialog",
};

type Pending = { action: "trash" | "purge"; taskIds: string[] };

export function EnvironmentsPage({ mode }: { mode: "files" | "trash" }) {
  const { t, locale } = useI18n();
  const { environments, errorKey, failures, busy, trash, restore, purge } =
    useEnvironments();
  const [pending, setPending] = useState<Pending | null>(null);
  const [focusedId, setFocusedId] = useState<string | null>(null);
  const [titles, setTitles] = useState<Map<string, string>>(new Map());
  const inTrash = mode === "trash";
  const shown = (environments ?? []).filter(
    (environment) => Boolean(environment.worktree.trashed_at) === inTrash,
  );
  const selectable = (environment: Environment) =>
    inTrash || environment.worktree.status === "ready";
  const focused = shown.find(
    (environment) => environment.task_id === focusedId,
  );
  const candidates = shown.filter(selectable).map((item) => item.task_id);

  async function run(
    action: (taskIds: string[]) => Promise<void>,
    ids: string[],
  ) {
    setPending(null);
    setTitles(new Map(shown.map((item) => [item.task_id, item.task_title])));
    await action(ids);
  }

  function statusClass(worktree: Environment["worktree"]) {
    return worktree.status === "ready"
      ? "border-ink text-ink"
      : worktree.status === "failed" || worktree.status === "missing"
        ? "border-error text-error"
        : "border-line text-muted";
  }

  function pill({ worktree }: Environment) {
    return (
      <span
        className={`rounded-full border px-2.5 py-0.5 text-xs font-semibold ${statusClass(worktree)}`}
      >
        {t(`tasks.environment.status.${worktree.status}`)}
      </span>
    );
  }

  function aside(environment: Environment) {
    const { worktree } = environment;
    if (inTrash && worktree.status === "ready" && worktree.trashed_at) {
      return (
        <span>
          {translateWith(locale, "trash.daysLeft", {
            days: daysLeft(worktree.trashed_at, Date.now()),
          })}
        </span>
      );
    }
    const label = t(`tasks.environment.status.${worktree.status}`);
    return (
      <span className="inline-flex items-center gap-1.5">
        <span
          aria-hidden
          className={`size-2 rounded-full ${worktree.status === "ready" ? "bg-success" : worktree.status === "failed" || worktree.status === "missing" ? "bg-error" : "bg-muted"}`}
        />
        {label}
      </span>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 gap-3">
      <section className="flex min-h-0 min-w-0 flex-1 flex-col gap-5 overflow-y-auto rounded-3xl border border-line bg-pill p-3">
        <div className="grid gap-2 px-4 pt-3">
          <h1 className="text-2xl leading-6 font-bold text-ink">
            {t(`${mode}.title`)}
          </h1>
          <p className="text-sm text-muted">{t(`${mode}.description`)}</p>
        </div>
        {errorKey && (
          <div className="px-4">
            <Alert message={translateError(locale, errorKey)} />
          </div>
        )}
        {failures.length > 0 && (
          <div
            role="alert"
            className="mx-4 grid gap-2 rounded-2xl border border-error p-4"
          >
            <p className="text-sm font-semibold text-error">
              {translateWith(locale, "files.failed", {
                count: failures.length,
              })}
            </p>
            <ul className="grid gap-1 text-sm">
              {failures.map((failure) => (
                <li key={failure.task_id} className="text-ink">
                  <span className="font-medium">
                    {titles.get(failure.task_id) ?? failure.task_id}
                  </span>
                  {": "}
                  {translateError(locale, failure.error)}
                </li>
              ))}
            </ul>
          </div>
        )}
        {environments &&
          (shown.length > 0 ? (
            <EnvironmentList
              label={t(`${mode}.title`)}
              environments={shown}
              focusedId={focused?.task_id ?? null}
              onFocus={setFocusedId}
              aside={aside}
            />
          ) : null)}
        <div className="sticky bottom-0 mt-auto flex flex-wrap items-center justify-end gap-2 rounded-full border border-line bg-surface p-2 shadow-2xl">
          {inTrash ? (
            <>
              <Button
                type="button"
                variant="destructive"
                size="pill"
                disabled={busy || candidates.length === 0}
                onClick={() =>
                  setPending({ action: "purge", taskIds: candidates })
                }
              >
                {t("trash.emptyTrash")}
              </Button>
            </>
          ) : (
            <>
              <Button
                type="button"
                variant="destructive"
                size="pill"
                disabled={busy || candidates.length === 0}
                onClick={() =>
                  setPending({ action: "trash", taskIds: candidates })
                }
              >
                {t("files.trashAll")}
              </Button>
            </>
          )}
        </div>
      </section>
      {focused ? (
        <EnvironmentDetail
          environment={focused}
          status={pill(focused)}
          onClose={() => setFocusedId(null)}
          actions={
            inTrash ? (
              <>
                <Button
                  type="button"
                  variant="ghost"
                  size="pill"
                  className="border-line"
                  disabled={busy}
                  onClick={() => run(restore, [focused.task_id])}
                >
                  {t("trash.restoreOne")}
                </Button>
                <Button
                  type="button"
                  variant="destructive"
                  size="pill"
                  disabled={busy}
                  onClick={() =>
                    setPending({ action: "purge", taskIds: [focused.task_id] })
                  }
                >
                  {t("trash.purgeDialog.confirm")}
                </Button>
              </>
            ) : (
              <Button
                type="button"
                variant="destructive"
                size="pill"
                disabled={busy || !selectable(focused)}
                onClick={() =>
                  setPending({ action: "trash", taskIds: [focused.task_id] })
                }
              >
                {t("files.trashDialog.confirm")}
              </Button>
            )
          }
        />
      ) : environments ? (
        shown.length === 0 &&
        (mode === "trash" ? <TrashEmptyCalendar /> : <EnvironmentsEmptyGuide />)
      ) : (
        <aside
          aria-hidden
          className="min-w-0 flex-1 rounded-3xl border border-line bg-pill"
        />
      )}
      {pending && (
        <ConfirmDialog
          title={translateWith(locale, `${dialogKeys[pending.action]}.title`, {
            count: pending.taskIds.length,
          })}
          body={t(`${dialogKeys[pending.action]}.body`)}
          confirmLabel={t(`${dialogKeys[pending.action]}.confirm`)}
          busy={busy}
          onConfirm={() =>
            run(pending.action === "trash" ? trash : purge, pending.taskIds)
          }
          onClose={() => setPending(null)}
        />
      )}
    </div>
  );
}
