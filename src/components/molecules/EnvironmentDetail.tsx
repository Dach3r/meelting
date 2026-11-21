import { useState, type ReactNode } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { formatBytes, formatDateTime } from "../../i18n/translate";
import { CloseIcon, FolderTileIcon } from "../atoms/Icons";
import type { Environment } from "../../hooks/useEnvironments";

type Props = {
  environment: Environment;
  status: ReactNode;
  actions: ReactNode;
  onClose: () => void;
};

function CopyButton({ value }: { value: string }) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      setCopied(false);
    }
  }

  return (
    <button
      type="button"
      onClick={copy}
      className="shrink-0 cursor-pointer rounded-full px-3 py-1 text-xs font-semibold text-muted transition-colors hover:bg-hi hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
    >
      {copied ? t("files.detail.copied") : t("files.detail.copy")}
    </button>
  );
}

type Row = { label: string; value: string; mono?: boolean; copy?: boolean };

function Group({ title, rows }: { title: string; rows: Row[] }) {
  return (
    <section className="grid gap-2">
      <h3 className="px-1 text-xs font-semibold tracking-wide text-muted uppercase">
        {title}
      </h3>
      <dl className="grid overflow-hidden rounded-2xl bg-hi text-sm">
        {rows.map((row) => (
          <div
            key={row.label}
            className="flex min-w-0 items-center justify-between gap-4 border-b border-line px-5 py-3 last:border-b-0"
          >
            <dt className="shrink-0 text-muted">{row.label}</dt>
            <dd className="flex min-w-0 items-center gap-2">
              {row.mono ? (
                <code className="min-w-0 text-right text-xs leading-relaxed break-all text-ink">
                  {row.value}
                </code>
              ) : (
                <span className="min-w-0 text-right text-ink">{row.value}</span>
              )}
              {row.copy && <CopyButton value={row.value} />}
            </dd>
          </div>
        ))}
      </dl>
    </section>
  );
}

type Step = { label: string; note: string; done: boolean };

function Lifecycle({ title, steps }: { title: string; steps: Step[] }) {
  return (
    <section className="grid gap-2">
      <h3 className="px-1 text-xs font-semibold tracking-wide text-muted uppercase">
        {title}
      </h3>
      <ol className="grid rounded-2xl bg-hi px-5 pt-5 pb-1">
        {steps.map((step, index) => (
          <li key={step.label} className="flex gap-4">
            <span className="flex flex-col items-center">
              <span
                aria-hidden
                className={`mt-1 size-3 shrink-0 rounded-full ${step.done ? "bg-success" : "border-2 border-muted"}`}
              />
              {index < steps.length - 1 && (
                <span aria-hidden className="my-1 w-0.5 flex-1 bg-line" />
              )}
            </span>
            <div className="grid gap-0.5 pb-5">
              <span
                className={`text-sm font-semibold ${step.done ? "text-ink" : "text-muted"}`}
              >
                {step.label}
              </span>
              <span className="text-sm text-muted">{step.note}</span>
            </div>
          </li>
        ))}
      </ol>
    </section>
  );
}

export function EnvironmentDetail({
  environment,
  status,
  actions,
  onClose,
}: Props) {
  const { t, locale, timeFormat } = useI18n();
  const { worktree } = environment;
  const date = (iso: string) => formatDateTime(locale, iso, timeFormat);
  const steps: Step[] = [
    {
      label: t("files.detail.created"),
      note: date(worktree.created_at),
      done: true,
    },
    {
      label: t("files.detail.updated"),
      note: date(worktree.updated_at),
      done: true,
    },
    {
      label: t("files.detail.trashed"),
      note: worktree.trashed_at
        ? date(worktree.trashed_at)
        : t("files.detail.trashHint"),
      done: Boolean(worktree.trashed_at),
    },
  ];
  return (
    <aside
      aria-label={environment.task_title}
      className="flex min-h-0 min-w-0 flex-1 flex-col gap-4 rounded-3xl border border-line bg-pill p-6"
    >
      <div className="flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto">
        <div className="flex items-start gap-4">
          <span className="grid size-16 shrink-0 place-items-center rounded-2xl bg-raised text-ink">
            <FolderTileIcon className="h-8 w-10" />
          </span>
          <div className="grid min-w-0 flex-1 justify-items-start gap-2">
            {status}
            <h2 className="text-xl leading-tight font-bold text-ink">
              {environment.task_title}
            </h2>
          </div>
          <button
            type="button"
            aria-label={t("teams.close")}
            onClick={onClose}
            className="grid size-10 shrink-0 cursor-pointer place-items-center rounded-full bg-raised text-muted transition-colors hover:bg-hi hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
          >
            <CloseIcon />
          </button>
        </div>
        <Lifecycle title={t("files.detail.lifecycle")} steps={steps} />
        <Group
          title={t("files.detail.git")}
          rows={[
            {
              label: t("files.detail.repository"),
              value: environment.repository_name,
            },
            { label: t("tasks.environment.base"), value: worktree.base_ref },
            {
              label: t("files.detail.commit"),
              value: worktree.base_commit_sha.slice(0, 12),
              mono: true,
              copy: true,
            },
            {
              label: t("tasks.environment.branch"),
              value: worktree.branch_name,
              mono: true,
              copy: true,
            },
          ]}
        />
        <Group
          title={t("files.detail.location")}
          rows={[
            {
              label: t("tasks.environment.path"),
              value: worktree.trash_path ?? worktree.path,
              mono: true,
              copy: true,
            },
          ]}
        />
        {worktree.copies.length > 0 && (
          <section className="grid gap-3">
            <h3 className="text-sm font-semibold text-ink">
              {t("tasks.environment.copy.copied")}
            </h3>
            <ul className="grid overflow-hidden rounded-2xl border border-line text-sm">
              {worktree.copies.map((copy) => (
                <li
                  key={copy.relative_path}
                  className="flex min-w-0 items-center justify-between gap-4 border-b border-line px-5 py-3 last:border-b-0"
                >
                  <code className="min-w-0 truncate text-ink">
                    {copy.relative_path}
                  </code>
                  <span className="shrink-0 text-xs text-muted">
                    {formatBytes(locale, copy.size_bytes)} ·{" "}
                    {formatDateTime(locale, copy.copied_at, timeFormat)}
                  </span>
                </li>
              ))}
            </ul>
            <p className="text-xs text-muted">
              {t("tasks.environment.copy.copiedHint")}
            </p>
          </section>
        )}
      </div>
      <div className="flex shrink-0 flex-wrap gap-2 border-t border-line pt-4">
        {actions}
      </div>
    </aside>
  );
}
