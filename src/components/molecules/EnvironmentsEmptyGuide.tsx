import { useI18n } from "../../i18n/I18nProvider";

const steps = ["first", "second", "third"] as const;

export function EnvironmentsEmptyGuide() {
  const { t } = useI18n();
  return (
    <aside
      aria-label={t("files.guide.title")}
      className="grid min-w-0 flex-1 place-items-center rounded-3xl border border-line bg-pill p-7"
    >
      <ol className="grid w-full max-w-[340px]">
        {steps.map((step, index) => (
          <li
            key={step}
            className="relative grid grid-cols-[30px_minmax(0,1fr)] gap-3 pb-[18px] last:pb-0"
          >
            {index < steps.length - 1 && (
              <span
                aria-hidden
                className="absolute top-[30px] bottom-0.5 left-3.5 w-px bg-line"
              />
            )}
            <span
              aria-hidden
              className="grid size-[30px] place-items-center rounded-full bg-primary font-mono text-xs font-semibold text-ink"
            >
              {index + 1}
            </span>
            <div>
              <p className="block text-[13px] font-bold text-ink">
                {t(`files.guide.${step}.title`)}
              </p>
              <p className="block text-xs text-muted">
                {t(`files.guide.${step}.body`)}
              </p>
            </div>
          </li>
        ))}
      </ol>
    </aside>
  );
}
