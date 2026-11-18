import { Cloud, HardDrive } from "lucide-react";
import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { useLlmProviders, type LlmProvider } from "../../hooks/useLlmProviders";
import { useLocalModels } from "../../hooks/useLocalModels";
import { Field } from "../atoms/Field";
import { SelectField } from "../atoms/SelectField";
import { Alert } from "./Alert";
import { SegmentedControl } from "./SegmentedControl";

const locations: LlmProvider["location"][] = ["cloud", "local"];

type Props = {
  provider: string;
  model: string;
  onChange: (provider: string, model: string) => void;
};

export function EnginePicker({ provider, model, onChange }: Props) {
  const { t } = useI18n();
  const providers = useLlmProviders();
  const current = providers.find((entry) => entry.key === provider);
  const [chosenLocation, setLocation] = useState<LlmProvider["location"] | null>(null);
  const location = chosenLocation ?? current?.location ?? "cloud";
  const [localKey, setLocalKey] = useState(provider);
  const localProviders = providers.filter((entry) => entry.location === "local");
  const localProvider = localProviders.find((entry) => entry.key === localKey) ?? localProviders[0];
  const detected = useLocalModels(location === "local");
  const localState = detected?.find((entry) => entry.key === localProvider?.key);
  const localModels = localState?.models ?? [];
  const modelGroups = providers
    .filter((entry) => entry.location === "cloud")
    .map((entry) => ({ id: entry.key, label: entry.name, icon: Cloud, options: entry.models }));
  const retired = current?.location === "cloud" && model !== "" && !current.models.includes(model);

  return (
    <>
      <SegmentedControl
        label={t("teams.agent.location")}
        options={locations.map((value) => ({ value, label: t(`teams.agent.locations.${value}`) }))}
        value={location}
        onChange={(value) => {
          setLocation(value);
          onChange("", "");
        }}
      />
      {retired && <Alert message={t("teams.agent.modelRetired")} />}
      {location === "cloud" && (
        <SelectField
          label={t("teams.agent.model")}
          searchPlaceholder={t("teams.agent.modelSearch")}
          emptyMessage={t("teams.agent.modelEmpty")}
          value={model}
          groups={modelGroups}
          onChange={(nextModel, nextProvider) => onChange(nextProvider, nextModel)}
          autoFocus
        />
      )}
      {location === "local" && localProvider && (
        <>
          <SegmentedControl
            label={t("teams.agent.runtime")}
            options={localProviders.map((entry) => ({ value: entry.key, label: entry.name }))}
            value={localProvider.key}
            onChange={(value) => {
              setLocalKey(value);
              onChange("", "");
            }}
          />
          {detected === null && <p className="text-sm text-muted">{t("teams.agent.localDetecting")}</p>}
          {localModels.length > 0 && (
            <SelectField
              key={localProvider.key}
              label={t("teams.agent.model")}
              searchPlaceholder={t("teams.agent.modelSearch")}
              emptyMessage={t("teams.agent.modelEmpty")}
              value={model}
              groups={[{ id: localProvider.key, label: localProvider.name, icon: HardDrive, options: localModels }]}
              onChange={(nextModel) => onChange(localProvider.key, nextModel)}
              autoFocus
            />
          )}
          {detected !== null && localModels.length === 0 && (
            <>
              <Alert message={t(localState?.running ? "teams.agent.localNoModels" : "teams.agent.localStopped")} />
              <Field
                label={t("teams.agent.modelName")}
                value={model}
                onChange={(nextModel) => onChange(localProvider.key, nextModel)}
                maxLength={200}
                icon={HardDrive}
              />
            </>
          )}
        </>
      )}
    </>
  );
}
