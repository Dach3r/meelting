import type { ReactNode } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { formatBytes, formatDateTime } from "../../i18n/translate";
import { FolderTileIcon } from "../atoms/Icons";
import type { Environment } from "../../hooks/useEnvironments";

type Props = {
  environment: Environment;
  status: ReactNode;
  actions: ReactNode;
};

export function EnvironmentDetail({ environment, status, actions }: Props) {
  const { t, locale, timeFormat } = useI18n();
  const { worktree } = environment;
  const short = [
    [t("files.detail.repository"), environment.repository_name],
    [t("tasks.environment.base"), worktree.base_ref],
    [t("files.detail.commit"), worktree.base_commit_sha.slice(0, 12)],
  ];
  const long = [
    [t("tasks.environment.branch"), worktree.branch_name],
    [t("tasks.environment.path"), worktree.trash_path ?? worktree.path],
  ];
  return (
    <aside
      aria-label={environment.task_title}
      className="flex min-h-0 w-104 shrink-0 flex-col gap-6 overflow-y-auto rounded-3xl border border-line bg-pill p-6"
    >
      <div className="flex items-start gap-4">
        <span className="grid size-16 shrink-0 place-items-center rounded-2xl bg-raised text-ink">
          <FolderTileIcon className="h-8 w-10" />
        </span>
        <div className="grid min-w-0 justify-items-start gap-2">
          {status}
          <h2 className="text-xl leading-tight font-bold text-ink">
            {environment.task_title}
          </h2>
        </div>
      </div>
      <div className="flex flex-wrap gap-2">{actions}</div>
      <dl className="grid grid-cols-2 gap-2 text-sm">
        {short.map(([label, value], index) => (
          <div
            key={label}
            className={`grid min-w-0 content-start gap-1 rounded-2xl bg-raised px-4 py-3 ${index === 0 ? "col-span-2" : ""}`}
          >
            <dt className="text-xs text-muted">{label}</dt>
            <dd>
              <code className="break-all text-ink">{value}</code>
            </dd>
          </div>
        ))}
        {long.map(([label, value]) => (
          <div
            key={label}
            className="col-span-2 grid min-w-0 content-start gap-1 rounded-2xl bg-raised px-4 py-3"
          >
            <dt className="text-xs text-muted">{label}</dt>
            <dd>
              <code className="text-xs leading-relaxed break-all text-ink">
                {value}
              </code>
            </dd>
          </div>
        ))}
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
    </aside>
  );
}
