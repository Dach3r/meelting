import { useI18n } from "../../i18n/I18nProvider";
import { translateError } from "../../i18n/translate";
import { useAgentTools } from "../../hooks/useAgentTools";
import { Switch } from "../atoms/Switch";
import { Alert } from "../molecules/Alert";
import { SettingsRow } from "../molecules/SettingsRow";

export function AgentToolsSetting() {
  const { t, locale } = useI18n();
  const { tools, errorKey, busy, change } = useAgentTools();
  const available = tools?.available !== false;
  return (
    <>
      <SettingsRow
        title={t("settings.agentTools.title")}
        description={t(
          available
            ? "settings.agentTools.description"
            : "settings.agentTools.unavailable",
        )}
      >
        <Switch
          label={t("settings.agentTools.title")}
          checked={tools?.enabled === true}
          onChange={change}
          disabled={tools === null || !available || busy}
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
