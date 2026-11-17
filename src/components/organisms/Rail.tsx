import { Activity, KeyRound, ListChecks, Plug, Settings, Users, Video } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { Logo } from "../atoms/Logo";
import { NavItem } from "../molecules/NavItem";

const items = [
  { key: "teams", icon: Users, current: true },
  { key: "rooms", icon: Video },
  { key: "tasks", icon: ListChecks },
  { key: "activity", icon: Activity },
  { key: "integrations", icon: Plug },
  { key: "apiKeys", icon: KeyRound },
  { key: "settings", icon: Settings },
];

export function Rail() {
  const { t } = useI18n();
  return (
    <aside className="flex w-64 shrink-0 flex-col gap-6 overflow-y-auto py-3 pr-3 text-muted">
      <div className="flex px-4 pt-1 pb-2">
        <Logo />
      </div>
      <nav aria-label={t("home.nav.label")} className="grid content-start gap-2">
        {items.map((item) => (
          <NavItem key={item.key} label={t(`home.nav.${item.key}`)} icon={item.icon} current={item.current} />
        ))}
      </nav>
    </aside>
  );
}
