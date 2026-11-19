import { useI18n } from "../../i18n/I18nProvider";

const days = Array.from({ length: 30 }, (_, index) => index);

function cellTone(day: number) {
  if (day === 0) return "border-accent bg-accent";
  if (day === days.length - 1) return "border-error bg-error";
  return "border-line bg-raised";
}

export function TrashEmptyCalendar() {
  const { t } = useI18n();
  return (
    <aside
      aria-label={t("trash.empty.title")}
      className="grid min-w-0 flex-1 place-items-center rounded-3xl border border-line bg-pill p-7"
    >
      <div className="flex flex-col items-center gap-5 text-center">
        <div aria-hidden className="grid grid-cols-10 gap-1.5">
          {days.map((day) => (
            <span
              key={day}
              className={`size-7 rounded-[9px] border ${cellTone(day)}`}
            />
          ))}
        </div>
        <ul className="flex flex-wrap justify-center gap-x-[18px] gap-y-2 text-xs text-muted">
          <li className="flex items-center gap-2">
            <span
              aria-hidden
              className="size-3 rounded border border-accent bg-accent"
            />
            {t("trash.empty.enters")}
          </li>
          <li className="flex items-center gap-2">
            <span
              aria-hidden
              className="size-3 rounded border border-error bg-error"
            />
            {t("trash.empty.deleted")}
          </li>
        </ul>
        <div>
          <p className="text-base font-semibold text-ink">
            {t("trash.empty.title")}
          </p>
          <p className="text-[13px] text-muted">{t("trash.empty.body")}</p>
        </div>
      </div>
    </aside>
  );
}
