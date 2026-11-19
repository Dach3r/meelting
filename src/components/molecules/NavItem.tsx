import type { LucideIcon } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";

export function NavItem({
  label,
  icon: Icon,
  current = false,
  onSelect,
}: {
  label: string;
  icon: LucideIcon;
  current?: boolean;
  onSelect?: () => void;
}) {
  const { t } = useI18n();
  const classes =
    "flex h-14 items-center gap-4 rounded-2xl px-4 text-base font-semibold group-data-collapsed/rail:justify-center group-data-collapsed/rail:px-0";
  const content = (
    <>
      <Icon className="size-6 shrink-0" aria-hidden="true" />
      <span className="whitespace-nowrap group-data-collapsed/rail:hidden">
        {label}
      </span>
    </>
  );
  if (current) {
    const currentClasses = `${classes} bg-ink/12 text-ink`;
    if (onSelect) {
      return (
        <button
          type="button"
          aria-current="page"
          onClick={onSelect}
          className={`${currentClasses} w-full cursor-pointer focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ink`}
        >
          {content}
        </button>
      );
    }
    return (
      <span aria-current="page" className={currentClasses}>
        {content}
      </span>
    );
  }
  if (onSelect) {
    return (
      <button
        type="button"
        onClick={onSelect}
        className={`${classes} w-full cursor-pointer hover:bg-ink/6 hover:text-ink focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ink`}
      >
        {content}
      </button>
    );
  }
  return (
    <span
      aria-disabled="true"
      title={t("home.soon")}
      className={`${classes} cursor-not-allowed opacity-50`}
    >
      {content}
    </span>
  );
}
