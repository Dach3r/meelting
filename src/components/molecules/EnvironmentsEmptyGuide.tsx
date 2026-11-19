import { useI18n } from "../../i18n/I18nProvider";

const steps = ["repository", "task", "environment"] as const;

export function EnvironmentsEmptyGuide() {
  const { t } = useI18n();
  return (
    <aside
      aria-label={t("files.guide.title")}
      className="grid min-w-0 flex-1 place-items-center rounded-3xl border border-line bg-pill p-7"
    >
      <ol className="grid w-full max-w-sm">
        {steps.map((step, index) => (
          <li
            key={step}
            className="relative grid grid-cols-[2rem_minmax(0,1fr)] gap-3 pb-5 last:pb-0"
          >
            {index < steps.length - 1 && (
              <span
                aria-hidden
                className="absolute top-8 bottom-0.5 left-4 w-px bg-line"
              />
            )}
            <span
              aria-hidden
              className="grid size-8 place-items-center rounded-full bg-primary text-xs font-semibold text-ink"
            >
              {index + 1}
            </span>
            <div className="grid gap-0.5">
              <p className="text-sm font-semibold text-ink">
                {t(`files.guide.${step}.title`)}
              </p>
              <p className="text-sm text-muted">
                {t(`files.guide.${step}.body`)}
              </p>
            </div>
          </li>
        ))}
      </ol>
    </aside>
  );
}
