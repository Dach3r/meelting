import { useState } from "react";
import { useEnvironments, type Environment } from "../../hooks/useEnvironments";
import { useI18n } from "../../i18n/I18nProvider";
import { translateError, translateWith } from "../../i18n/translate";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { ConfirmDialog } from "../molecules/ConfirmDialog";
import { EnvironmentDetail } from "../molecules/EnvironmentDetail";
import { EnvironmentsEmptyGuide } from "../molecules/EnvironmentsEmptyGuide";
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
  const [focusedId, setFocusedId] = useState<string | null>(null);
  const [titles, setTitles] = useState<Map<string, string>>(new Map());
  const inTrash = mode === "trash";
  const shown = (environments ?? []).filter(
    (environment) => Boolean(environment.worktree.trashed_at) === inTrash,
  );
  const selectable = (environment: Environment) =>
    inTrash || environment.worktree.status === "ready";
  const focused =
    shown.find((environment) => environment.task_id === focusedId) ?? shown[0];
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
      <section
        className="flex min-h-0 w-100 shrink-0 flex-col gap-5 overflow-y-auto rounded-3xl border border-line bg-pill p-3"
      >
        <div className="grid gap-2 px-4 pt-3">
          <h1 className="text-2xl leading-6 font-bold text-ink">{t(`${mode}.title`)}</h1>
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
          (focused ? (
            <EnvironmentList
              label={t(`${mode}.title`)}
              environments={shown}
              selected={selected}
              selectable={selectable}
              focusedId={focused.task_id}
              onFocus={setFocusedId}
              onToggle={toggle}
              onToggleAll={toggleAll}
              aside={aside}
            />
          ) : (
            inTrash && (
              <p className="mx-4 rounded-2xl border border-line px-4 py-6 text-center text-sm text-muted">
                {t("trash.empty")}
              </p>
            )
          ))}
        <div className="mt-auto grid gap-2 pt-2">
          {inTrash ? (
            <>
              <Button
                type="button"
                variant="ghost"
                size="pill"
                className="w-full border-line"
                disabled={busy || chosen.length === 0}
                onClick={() => run(restore, chosen)}
              >
                {translateWith(locale, "trash.restore", {
                  count: chosen.length,
                })}
              </Button>
              <Button
                type="button"
                variant="danger"
                size="pill"
                className="w-full"
                disabled={busy || chosen.length === 0}
                onClick={() => setPending({ action: "purge", taskIds: chosen })}
              >
                {translateWith(locale, "trash.purge", { count: chosen.length })}
              </Button>
              <Button
                type="button"
                variant="destructive"
                size="pill"
                className="w-full"
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
                className="w-full"
                disabled={busy || chosen.length === 0}
                onClick={() => setPending({ action: "trash", taskIds: chosen })}
              >
                {translateWith(locale, "files.trash", { count: chosen.length })}
              </Button>
              <Button
                type="button"
                variant="destructive"
                size="pill"
                className="w-full"
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
      ) : !inTrash && environments ? (
        <EnvironmentsEmptyGuide />
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
