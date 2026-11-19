import { FolderOpen } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { translateError } from "../../i18n/translate";
import { useDataFolder } from "../../hooks/useDataFolder";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { SettingsRow } from "../molecules/SettingsRow";

export function DataFolderSetting() {
  const { t, locale } = useI18n();
  const { errorKey, busy, open } = useDataFolder();
  return (
    <>
      <SettingsRow
        title={t("settings.dataFolder.title")}
        description={t("settings.dataFolder.description")}
      >
        <Button
          type="button"
          variant="ghost"
          size="pill"
          className="border-line"
          disabled={busy}
          onClick={open}
        >
          <FolderOpen aria-hidden className="size-4" />
          {t("settings.dataFolder.open")}
        </Button>
      </SettingsRow>
      {errorKey && (
        <div className="px-6 pb-5">
          <Alert message={translateError(locale, errorKey)} />
        </div>
      )}
    </>
  );
}
