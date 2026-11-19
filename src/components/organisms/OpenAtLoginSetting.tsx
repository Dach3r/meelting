import { useI18n } from "../../i18n/I18nProvider";
import { translateError } from "../../i18n/translate";
import { useOpenAtLogin } from "../../hooks/useOpenAtLogin";
import { Switch } from "../atoms/Switch";
import { Alert } from "../molecules/Alert";
import { SettingsRow } from "../molecules/SettingsRow";

export function OpenAtLoginSetting() {
  const { t, locale } = useI18n();
  const { enabled, errorKey, busy, change } = useOpenAtLogin();
  return (
    <>
      <SettingsRow
        title={t("settings.openAtLogin.title")}
        description={t("settings.openAtLogin.description")}
      >
        <Switch
          label={t("settings.openAtLogin.title")}
          checked={enabled === true}
          onChange={change}
          disabled={enabled === null || busy}
        />
      </SettingsRow>
      {errorKey && (
        <div className="px-6 pb-5">
          <Alert message={translateError(locale, errorKey)} />
        </div>
      )}
    </>
  );
}
