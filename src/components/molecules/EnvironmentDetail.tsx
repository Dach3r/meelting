import type { ReactNode } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { formatBytes, formatDateTime } from "../../i18n/translate";
import type { Environment } from "../../hooks/useEnvironments";

type Props = {
  environment: Environment;
  status: ReactNode;
  hint: string;
  actions: ReactNode;
};

export function EnvironmentDetail({
  environment,
  status,
  hint,
  actions,
}: Props) {
  const { t, locale } = useI18n();
  const { worktree } = environment;
  const rows = [
    [t("files.detail.repository"), environment.repository_name],
    [t("tasks.environment.branch"), worktree.branch_name],
    [t("tasks.environment.base"), worktree.base_ref],
    [t("files.detail.commit"), worktree.base_commit_sha.slice(0, 12)],
    [t("tasks.environment.path"), worktree.trash_path ?? worktree.path],
  ];
  return (
    <aside
      aria-label={environment.task_title}
      className="flex min-h-0 min-w-0 flex-1 flex-col gap-7 overflow-y-auto rounded-3xl border border-line bg-pill p-10"
    >
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div className="grid justify-items-start gap-3">
          {status}
          <h2 className="text-3xl font-bold text-ink">
            {environment.task_title}
          </h2>
        </div>
        <div className="flex flex-wrap gap-2">{actions}</div>
      </div>
      <dl className="grid grid-cols-[9rem_minmax(0,1fr)] overflow-hidden rounded-2xl border border-line text-sm">
        {rows.map(([label, value], index) => {
          const edge = index === 0 ? "" : "border-t border-line";
          return (
          <div key={label} className="contents">
            <dt className={`${edge} bg-shell px-5 py-4 text-muted`}>
              {label}
            </dt>
            <dd className={`${edge} px-5 py-4`}>
              <code className="break-all text-ink select-text">{value}</code>
            </dd>
          </div>
          );
        })}
      </dl>
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
                <code className="min-w-0 truncate text-ink select-text">
                  {copy.relative_path}
                </code>
                <span className="shrink-0 text-xs text-muted">
                  {formatBytes(locale, copy.size_bytes)} ·{" "}
                  {formatDateTime(locale, copy.copied_at)}
                </span>
              </li>
            ))}
          </ul>
          <p className="text-xs text-muted">
            {t("tasks.environment.copy.copiedHint")}
          </p>
        </section>
      )}
      <p className="rounded-2xl bg-shell px-5 py-4 text-sm leading-relaxed text-muted">
        {hint}
      </p>
    </aside>
  );
}
