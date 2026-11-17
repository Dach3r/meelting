import { useI18n } from "../../i18n/I18nProvider";
import type { Locale } from "../../i18n/translate";
import { SegmentedControl } from "./SegmentedControl";

const options = (["es", "en"] as const).map((locale) => ({
  value: locale,
  label: locale.toUpperCase(),
  lang: locale,
}));

export function LocaleSwitcher({
  value,
  onChange,
  disabled,
}: {
  value: Locale;
  onChange: (locale: Locale) => void;
  disabled?: boolean;
}) {
  const { t } = useI18n();
  return (
    <SegmentedControl
      label={t("locale.label")}
      options={options}
      value={value}
      onChange={onChange}
      disabled={disabled}
    />
  );
}
