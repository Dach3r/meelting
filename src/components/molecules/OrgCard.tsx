import { ChevronDown, ChevronUp, Plus } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { Avatar } from "../atoms/Avatar";
import { CardSelect } from "../atoms/CardSelect";

type Props = {
  seed: string;
  name: string;
  role: string;
  asHeading?: boolean;
  count?: number;
  expanded?: boolean;
  onToggle?: () => void;
  addLabel?: string;
  onAdd?: () => void;
  selected?: boolean;
  onSelect?: () => void;
};

export function OrgCard({ seed, name, role, asHeading = false, count = 0, expanded = true, onToggle, addLabel, onAdd, selected = false, onSelect }: Props) {
  const { t } = useI18n();
  const Name = asHeading ? "h2" : "p";
  return (
    <div className={`group relative flex h-16 w-60 items-center gap-3 rounded-2xl border bg-raised p-2 pr-3 transition-colors hover:bg-hi ${selected ? "border-accent" : "border-line"}`}>
      <Avatar seed={seed} label={name} className="size-11 rounded-full" />
      <div className="min-w-0 flex-1 leading-tight">
        <Name className="truncate text-base font-bold tracking-tight text-ink">{name}</Name>
        <p className="truncate text-sm font-medium text-muted">{role}</p>
      </div>
      {count > 0 && (
        <button
          type="button"
          aria-expanded={expanded}
          aria-label={`${t("teams.chart.toggle")} ${name}`}
          onClick={onToggle}
          className="relative z-10 flex h-7 min-w-7 cursor-pointer items-center justify-center gap-0.5 rounded-full bg-shell px-2.5 text-sm font-semibold tabular-nums text-ink transition-colors hover:bg-hi focus-visible:outline-2 focus-visible:outline-ink"
        >
          {count}
          {expanded ? (
            <ChevronUp className="size-3" aria-hidden="true" />
          ) : (
            <ChevronDown className="size-3" aria-hidden="true" />
          )}
        </button>
      )}
      {onSelect && <CardSelect label={name} selected={selected} onSelect={onSelect} />}
      {onAdd && (
        <button
          type="button"
          aria-label={addLabel}
          title={addLabel}
          onClick={onAdd}
          className="absolute bottom-0 left-1/2 z-10 flex size-7 -translate-x-1/2 translate-y-1/2 cursor-pointer items-center justify-center rounded-full bg-ink text-surface opacity-0 shadow-md transition hover:scale-110 focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink group-hover:opacity-100"
        >
          <Plus className="size-4" aria-hidden="true" />
        </button>
      )}
    </div>
  );
}
