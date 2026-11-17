import type { LucideIcon } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";

export function NavItem({
  label,
  icon: Icon,
  current = false,
}: {
  label: string;
  icon: LucideIcon;
  current?: boolean;
}) {
  const { t } = useI18n();
  const classes = "flex h-14 items-center gap-4 rounded-2xl px-4 text-base font-semibold group-data-collapsed/rail:justify-center group-data-collapsed/rail:px-0";
  if (current) {
    return (
      <span aria-current="page" className={`${classes} bg-ink/12 text-ink`}>
        <Icon className="size-6 shrink-0" aria-hidden="true" />
        <span className="whitespace-nowrap group-data-collapsed/rail:hidden">{label}</span>
      </span>
    );
  }
  return (
    <span aria-disabled="true" title={t("home.soon")} className={`${classes} cursor-not-allowed opacity-50`}>
      <Icon className="size-6 shrink-0" aria-hidden="true" />
      <span className="whitespace-nowrap group-data-collapsed/rail:hidden">{label}</span>
    </span>
  );
}
