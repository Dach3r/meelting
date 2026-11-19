import { useI18n } from "../../i18n/I18nProvider";
import { translateWith } from "../../i18n/translate";

type Props = {
  name: string;
  detecting: boolean;
  models: string[] | null;
};

export function LocalProviderRow({ name, detecting, models }: Props) {
  const { locale, t } = useI18n();
  const label = detecting
    ? t("providers.local.detecting")
    : models === null
      ? t("providers.local.stopped")
      : models.length === 0
        ? t("providers.local.noModels")
        : models.length === 1
          ? t("providers.local.runningOne")
          : translateWith(locale, "providers.local.running", {
              count: models.length,
            });
  return (
    <div className="flex items-center gap-6 px-6 py-5">
      <div className="min-w-0 flex-1">
        <h3 className="text-base font-semibold text-ink">{name}</h3>
        <p className="flex items-center gap-2 text-sm text-muted">
          <span
            aria-hidden
            className={`size-2 shrink-0 rounded-full ${models?.length ? "bg-accent" : "bg-muted"}`}
          />
          {label}
        </p>
      </div>
    </div>
  );
}
