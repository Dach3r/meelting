import { X } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { Avatar } from "../atoms/Avatar";

type Props = { seed: string; name: string; role: string; teamName: string; onClose: () => void };

export function AgentDrawer({ seed, name, role, teamName, onClose }: Props) {
  const { t } = useI18n();
  const rows = [
    [t("teams.agent.team"), teamName],
    [t("teams.agent.role"), role],
  ];
  return (
    <aside
      aria-label={name}
      onClick={(event) => event.stopPropagation()}
      className="absolute top-4 right-4 bottom-20 flex w-72 flex-col gap-4 rounded-2xl border border-line bg-surface p-4 shadow-sm"
    >
      <button
        type="button"
        aria-label={t("teams.close")}
        onClick={onClose}
        className="absolute top-3 right-3 grid size-8 cursor-pointer place-items-center rounded-lg text-muted transition-colors hover:bg-primary hover:text-ink focus-visible:outline-2 focus-visible:outline-ink"
      >
        <X className="size-4" aria-hidden="true" />
      </button>
      <div className="flex items-center gap-3 pr-8">
        <Avatar seed={seed} label={name} className="size-12 rounded-full text-lg" />
        <div className="min-w-0 leading-tight">
          <h3 className="truncate text-base font-semibold tracking-tight text-ink">{name}</h3>
          <p className="truncate text-sm font-medium text-muted">{role}</p>
        </div>
      </div>
      <dl>
        {rows.map(([label, value]) => (
          <div key={label} className="flex justify-between gap-4 border-b border-line py-2 text-sm last:border-0">
            <dt className="text-muted">{label}</dt>
            <dd className="truncate font-medium text-ink">{value}</dd>
          </div>
        ))}
      </dl>
    </aside>
  );
}
