import { X, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { Avatar, hueOf } from "../atoms/Avatar";

type Row = { Icon: LucideIcon; label: string; value: string };

type Props = { seed: string; name: string; subtitle: string; rows: Row[]; children?: ReactNode; actions: ReactNode; onClose: () => void };

export function OrgDrawer({ seed, name, subtitle, rows, children, actions, onClose }: Props) {
  const { t } = useI18n();
  const hue = hueOf(seed);
  return (
    <aside
      aria-label={name}
      onClick={(event) => event.stopPropagation()}
      className="absolute top-4 right-4 bottom-4 flex w-80 flex-col overflow-hidden rounded-3xl border border-line bg-surface shadow-2xl shadow-black/30"
    >
      <div
        style={{ backgroundImage: `linear-gradient(135deg, hsl(${hue} 45% 30%), hsl(${(hue + 50) % 360} 55% 18%))` }}
        className="relative h-24 shrink-0"
      >
        <button
          type="button"
          aria-label={t("teams.close")}
          onClick={onClose}
          className="absolute top-3 right-3 grid size-8 cursor-pointer place-items-center rounded-full bg-black/30 text-white/80 backdrop-blur transition-colors hover:bg-black/50 hover:text-white focus-visible:outline-2 focus-visible:outline-white"
        >
          <X className="size-4" aria-hidden="true" />
        </button>
      </div>
      <div className="flex flex-1 flex-col gap-4 px-5 pb-5">
        <div className="-mt-10 grid gap-2">
          <Avatar seed={seed} label={name} className="relative size-20 rounded-full text-3xl ring-4 ring-surface" />
          <div className="min-w-0 leading-tight">
            <h3 className="truncate text-xl font-bold tracking-tight text-ink">{name}</h3>
            <p className="mt-1 truncate text-sm font-medium text-muted">{subtitle}</p>
          </div>
        </div>
        <dl className="grid gap-2">
          {rows.map(({ Icon, label, value }) => (
            <div key={label} className="flex items-center gap-3 rounded-2xl bg-raised p-3">
              <span className="grid size-9 shrink-0 place-items-center rounded-xl bg-surface text-muted">
                <Icon className="size-4" aria-hidden="true" />
              </span>
              <div className="min-w-0 leading-tight">
                <dt className="text-xs font-medium text-muted">{label}</dt>
                <dd className="mt-0.5 truncate text-sm font-semibold text-ink">{value}</dd>
              </div>
            </div>
          ))}
        </dl>
        {children}
        <div className="mt-auto grid gap-2">{actions}</div>
      </div>
    </aside>
  );
}
