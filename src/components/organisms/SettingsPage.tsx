import { useI18n } from "../../i18n/I18nProvider";
import type { Locale } from "../../i18n/translate";
import type { Theme } from "../../hooks/useSession";
import { Alert } from "../molecules/Alert";
import { LocaleSwitcher } from "../molecules/LocaleSwitcher";
import { ThemeSwitcher } from "../molecules/ThemeSwitcher";

type Props = {
  locale: Locale;
  theme: Theme;
  busy: boolean;
  errorMessage: string | null;
  onChangeLocale: (locale: Locale) => void;
  onChangeTheme: (theme: Theme) => void;
};

export function SettingsPage({ locale, theme, busy, errorMessage, onChangeLocale, onChangeTheme }: Props) {
  const { t } = useI18n();
  return (
    <div className="mx-auto grid w-full max-w-2xl content-start gap-10">
      {errorMessage && <Alert message={errorMessage} />}
      <section className="grid gap-4">
        <div>
          <h2 className="text-lg font-bold text-ink">{t("settings.language.title")}</h2>
          <p className="text-sm text-muted">{t("settings.language.description")}</p>
        </div>
        <LocaleSwitcher value={locale} onChange={onChangeLocale} disabled={busy} />
      </section>
      <section className="grid gap-4">
        <div>
          <h2 className="text-lg font-bold text-ink">{t("settings.theme.title")}</h2>
          <p className="text-sm text-muted">{t("settings.theme.description")}</p>
        </div>
        <ThemeSwitcher value={theme} onChange={onChangeTheme} disabled={busy} />
      </section>
    </div>
  );
}
