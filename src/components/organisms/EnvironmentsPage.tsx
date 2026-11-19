import { useState } from "react";
import { useEnvironments, type Environment } from "../../hooks/useEnvironments";
import { useI18n } from "../../i18n/I18nProvider";
import { translateError, translateWith } from "../../i18n/translate";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { ConfirmDialog } from "../molecules/ConfirmDialog";
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
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [pending, setPending] = useState<Pending | null>(null);
  const [titles, setTitles] = useState<Map<string, string>>(new Map());
  const inTrash = mode === "trash";
  const shown = (environments ?? []).filter(
    (environment) => Boolean(environment.worktree.trashed_at) === inTrash,
  );
  const selectable = (environment: Environment) =>
    inTrash || environment.worktree.status === "ready";
  const candidates = shown.filter(selectable).map((item) => item.task_id);
  const chosen = candidates.filter((taskId) => selected.has(taskId));

  function toggle(taskId: string) {
    setSelected((current) => {
      const next = new Set(current);
      if (!next.delete(taskId)) next.add(taskId);
      return next;
    });
  }

  function toggleAll() {
    setSelected(
      chosen.length === candidates.length ? new Set() : new Set(candidates),
    );
  }

  async function run(
    action: (taskIds: string[]) => Promise<void>,
    ids: string[],
  ) {
    setPending(null);
    setTitles(new Map(shown.map((item) => [item.task_id, item.task_title])));
    await action(ids);
    setSelected(new Set());
  }

  function aside(environment: Environment) {
    const { worktree } = environment;
    if (inTrash && worktree.status === "ready" && worktree.trashed_at) {
      return (
        <span className="text-xs font-medium text-muted">
          {translateWith(locale, "trash.daysLeft", {
            days: daysLeft(worktree.trashed_at, Date.now()),
          })}
        </span>
      );
    }
    return (
      <span
        className={`rounded-full border px-2.5 py-0.5 text-xs font-semibold ${worktree.status === "ready" ? "border-ink text-ink" : worktree.status === "failed" || worktree.status === "missing" ? "border-error text-error" : "border-line text-muted"}`}
      >
        {t(`tasks.environment.status.${worktree.status}`)}
      </span>
    );
  }

  return (
    <div className="grid content-start gap-6">
      <div>
        <h1 className="text-2xl font-bold text-ink">{t(`${mode}.title`)}</h1>
        <p className="text-sm text-muted">{t(`${mode}.description`)}</p>
      </div>
      {errorKey && <Alert message={translateError(locale, errorKey)} />}
      {failures.length > 0 && (
        <div
          role="alert"
          className="grid gap-2 rounded-2xl border border-error p-4"
        >
          <p className="text-sm font-semibold text-error">
            {translateWith(locale, "files.failed", { count: failures.length })}
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
      <div className="flex flex-wrap gap-2">
        {inTrash ? (
          <>
            <Button
              type="button"
              variant="ghost"
              size="pill"
              className="border-line"
              disabled={busy || chosen.length === 0}
              onClick={() => run(restore, chosen)}
            >
              {translateWith(locale, "trash.restore", { count: chosen.length })}
            </Button>
            <Button
              type="button"
              variant="danger"
              size="pill"
              disabled={busy || chosen.length === 0}
              onClick={() => setPending({ action: "purge", taskIds: chosen })}
            >
              {translateWith(locale, "trash.purge", { count: chosen.length })}
            </Button>
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
              variant="danger"
              size="pill"
              disabled={busy || chosen.length === 0}
              onClick={() => setPending({ action: "trash", taskIds: chosen })}
            >
              {translateWith(locale, "files.trash", { count: chosen.length })}
            </Button>
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
      {environments &&
        (shown.length === 0 ? (
          <p className="rounded-2xl border border-line px-4 py-6 text-center text-sm text-muted">
            {t(`${mode}.empty`)}
          </p>
        ) : (
          <EnvironmentList
            label={t(`${mode}.title`)}
            environments={shown}
            selected={selected}
            selectable={selectable}
            onToggle={toggle}
            onToggleAll={toggleAll}
            aside={aside}
          />
        ))}
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
