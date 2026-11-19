import { useI18n } from "../../i18n/I18nProvider";
import type { UiScale } from "../../hooks/useSession";
import { SegmentedControl } from "./SegmentedControl";

const scales: UiScale[] = [90, 100, 110, 125, 150];

export function UiScaleSwitcher({
  value,
  onChange,
  disabled,
}: {
  value: UiScale;
  onChange: (uiScale: UiScale) => void;
  disabled?: boolean;
}) {
  const { t } = useI18n();
  const options = scales.map((scale) => ({ value: String(scale), label: `${scale}%` }));
  return (
    <SegmentedControl
      label={t("settings.uiScale.title")}
      options={options}
      value={String(value)}
      onChange={(selected) => onChange(Number(selected) as UiScale)}
      disabled={disabled}
    />
  );
}
