import { useI18n } from "../../i18n/I18nProvider";
import type { Locale } from "../../i18n/translate";

const locales: Locale[] = ["es", "en"];

export function LocaleSwitcher({
  value,
  onChange,
}: {
  value: Locale;
  onChange: (locale: Locale) => void;
}) {
  const { t } = useI18n();
  return (
    <nav aria-label={t("locale.label")} className="flex w-fit items-center gap-1 rounded-full bg-shell p-1 text-sm">
      {locales.map((locale) => (
        <button
          key={locale}
          type="button"
          lang={locale}
          aria-current={locale === value ? "true" : undefined}
          onClick={() => onChange(locale)}
          className={`cursor-pointer rounded-full px-3 py-1 tracking-wider ${locale === value ? "bg-ink font-semibold text-surface" : "font-bold text-muted hover:text-ink"}`}
        >
          {locale.toUpperCase()}
        </button>
      ))}
    </nav>
  );
}
