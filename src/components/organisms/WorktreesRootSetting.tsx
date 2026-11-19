import { FolderOpen } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { translateError } from "../../i18n/translate";
import { useWorktreesRoot } from "../../hooks/useWorktreesRoot";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { SettingsRow } from "../molecules/SettingsRow";

export function WorktreesRootSetting() {
  const { t, locale } = useI18n();
  const { root, errorKey, busy, changeRoot } = useWorktreesRoot();
  return (
    <>
      <SettingsRow
        title={t("settings.worktrees.title")}
        description={t("settings.worktrees.description")}
      >
        <Button
          type="button"
          variant="ghost"
          size="pill"
          className="border-line"
          disabled={busy}
          onClick={changeRoot}
        >
          <FolderOpen aria-hidden className="size-4" />
          {t("settings.worktrees.change")}
        </Button>
      </SettingsRow>
      {root && (
        <div className="grid gap-1 px-6 pb-5">
          <p className="text-xs font-medium text-muted">
            {root.custom
              ? t("settings.worktrees.custom")
              : t("settings.worktrees.default")}
          </p>
          <code className="text-sm break-all text-ink select-text">
            {root.path}
          </code>
          <p
            className={`text-sm ${root.issue ? "text-error" : "text-muted"}`}
          >
            {root.issue
              ? t(`settings.worktrees.issues.${root.issue}`)
              : t("settings.worktrees.ok")}
          </p>
        </div>
      )}
      {errorKey && (
        <div className="px-6 pb-5">
          <Alert message={translateError(locale, errorKey)} />
        </div>
      )}
    </>
  );
}
