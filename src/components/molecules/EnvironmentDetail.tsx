import type { ReactNode } from "react";
import { useI18n } from "../../i18n/I18nProvider";
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
  const { t } = useI18n();
  const { worktree } = environment;
  const rows = [
    [t("files.detail.repository"), environment.repository_name],
    [t("tasks.environment.branch"), worktree.branch_name],
    [t("tasks.environment.base"), worktree.base_ref],
  ];
  return (
    <aside
      aria-label={environment.task_title}
      className="grid content-start gap-5 rounded-2xl border border-line p-6"
    >
      <div className="grid justify-items-start gap-2">
        {status}
        <h2 className="text-xl font-bold text-ink">{environment.task_title}</h2>
      </div>
      <dl className="grid gap-4 border-y border-line py-4 text-sm">
        {rows.map(([label, value]) => (
          <div key={label} className="grid gap-1">
            <dt className="text-xs font-medium text-muted uppercase">
              {label}
            </dt>
            <dd>
              <code className="break-all text-ink select-text">{value}</code>
            </dd>
          </div>
        ))}
        <div className="grid gap-1">
          <dt className="text-xs font-medium text-muted uppercase">
            {t("tasks.environment.path")}
          </dt>
          <dd>
            <code className="break-all text-ink select-text">
              {worktree.trash_path ?? worktree.path}
            </code>
          </dd>
        </div>
      </dl>
      <div className="flex flex-wrap gap-2">{actions}</div>
      <p className="text-sm text-muted">{hint}</p>
    </aside>
  );
}
