import type { LucideIcon } from "lucide-react";
import {
  Activity,
  ChevronsLeft,
  ChevronsRight,
  FolderOpen,
  KeyRound,
  ListChecks,
  Plug,
  Settings,
  Trash2,
  Users,
  Video,
} from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { Logo } from "../atoms/Logo";
import { NavItem } from "../molecules/NavItem";

export type View =
  "teams" | "rooms" | "tasks" | "files" | "trash" | "providers" | "settings";

const items: { key: string; icon: LucideIcon; view?: View }[] = [
  { key: "teams", icon: Users, view: "teams" },
  { key: "rooms", icon: Video, view: "rooms" },
  { key: "tasks", icon: ListChecks, view: "tasks" },
  { key: "files", icon: FolderOpen, view: "files" },
  { key: "activity", icon: Activity },
  { key: "integrations", icon: Plug },
  { key: "providers", icon: KeyRound, view: "providers" },
  { key: "settings", icon: Settings, view: "settings" },
];

type Props = {
  collapsed: boolean;
  onToggle: () => void;
  view: View;
  onSelectView: (view: View) => void;
};

export function Rail({ collapsed, onToggle, view, onSelectView }: Props) {
  const { t } = useI18n();
  return (
    <aside className="flex flex-col gap-6 overflow-x-hidden overflow-y-auto py-3 pr-3 text-muted on-rail">
      <div className="flex px-4 pt-1 pb-2 group-data-collapsed/rail:justify-center group-data-collapsed/rail:px-0">
        <Logo />
      </div>
      <nav
        aria-label={t("home.nav.label")}
        className="grid content-start gap-2"
      >
        {items.map((item) => (
          <NavItem
            key={item.key}
            label={t(`home.nav.${item.key}`)}
            icon={item.icon}
            collapsed={collapsed}
            current={item.view === view}
            onSelect={item.view ? () => onSelectView(item.view!) : undefined}
          />
        ))}
      </nav>
      <div className="mt-auto grid gap-2">
        <NavItem
          label={t("home.nav.trash")}
          icon={Trash2}
          collapsed={collapsed}
          current={view === "trash"}
          onSelect={() => onSelectView("trash")}
        />
        <button
          type="button"
          aria-expanded={!collapsed}
          aria-label={t("home.rail.collapse")}
          onClick={onToggle}
          className="flex h-14 shrink-0 cursor-pointer items-center gap-4 rounded-2xl px-4 text-base font-semibold hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink group-data-collapsed/rail:justify-center group-data-collapsed/rail:px-0"
        >
          {collapsed ? (
            <ChevronsRight className="size-6 shrink-0" aria-hidden="true" />
          ) : (
            <ChevronsLeft className="size-6 shrink-0" aria-hidden="true" />
          )}
          <span className="whitespace-nowrap group-data-collapsed/rail:hidden">
            {t("home.rail.collapse")}
          </span>
        </button>
      </div>
    </aside>
  );
}
