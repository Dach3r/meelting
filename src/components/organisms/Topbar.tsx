import { Search } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { WorkspaceSwitcher, type WorkspaceSwitcherProps } from "../molecules/WorkspaceSwitcher";

export function Topbar(props: WorkspaceSwitcherProps) {
  const { t } = useI18n();
  return (
    <header className="relative z-10 flex h-22 shrink-0 items-start gap-3 bg-shell px-8 pt-4 pb-3">
      <WorkspaceSwitcher {...props} />
      <label className="flex h-15 min-w-64 flex-1 items-center gap-4 rounded-full bg-surface pr-6 pl-5 text-muted transition-colors focus-within:bg-raised">
        <Search className="size-6 shrink-0" aria-hidden="true" />
        <input
          type="search"
          placeholder={t("home.topbar.search")}
          aria-label={t("home.topbar.search")}
          className="min-w-0 flex-1 bg-transparent text-base text-ink outline-none placeholder:text-muted"
        />
      </label>
    </header>
  );
}
