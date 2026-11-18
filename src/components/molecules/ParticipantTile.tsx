import { useI18n } from "../../i18n/I18nProvider";
import { Avatar } from "../atoms/Avatar";

export function ParticipantTile({ name, role, thinking = false }: { name: string; role: string; thinking?: boolean }) {
  const { t } = useI18n();
  return (
    <li className="flex flex-col items-center gap-3 rounded-2xl border border-line bg-surface p-5 text-center">
      <Avatar seed={name} label={name} className={`size-14 rounded-2xl text-xl ${thinking ? "animate-pulse" : ""}`} />
      <div className="grid min-w-0 gap-0.5">
        <p className="truncate font-semibold text-ink">{name}</p>
        <p className="truncate text-xs text-muted">{role}</p>
        {thinking && <p className="text-xs font-medium text-accent">{t("agentRun.thinking")}</p>}
      </div>
    </li>
  );
}
