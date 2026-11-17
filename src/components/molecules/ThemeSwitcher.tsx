import { useI18n } from "../../i18n/I18nProvider";
import type { Theme } from "../../hooks/useSession";
import { SegmentedControl } from "./SegmentedControl";

const themes: Theme[] = ["system", "light", "dark"];

export function ThemeSwitcher({
  value,
  onChange,
  disabled,
}: {
  value: Theme;
  onChange: (theme: Theme) => void;
  disabled?: boolean;
}) {
  const { t } = useI18n();
  const options = themes.map((theme) => ({ value: theme, label: t(`settings.theme.${theme}`) }));
  return (
    <SegmentedControl
      label={t("settings.theme.title")}
      options={options}
      value={value}
      onChange={onChange}
      disabled={disabled}
    />
  );
}
