import { Avatar } from "../atoms/Avatar";
import { CardSelect } from "../atoms/CardSelect";

type Props = { seed: string; name: string; role: string; selected?: boolean; onSelect?: () => void };

export function OrgAgentCard({ seed, name, role, selected = false, onSelect }: Props) {
  return (
    <div
      className={`relative flex h-14 w-50 items-center gap-2.5 rounded-2xl border bg-raised p-2 pr-3 transition-colors hover:bg-hi ${selected ? "border-accent" : "border-line"}`}
    >
      <Avatar seed={seed} label={name} className="size-9 rounded-full" />
      <div className="min-w-0 leading-tight">
        <p className="truncate text-sm font-semibold tracking-tight text-ink">{name}</p>
        <p className="truncate text-xs font-medium text-muted">{role}</p>
      </div>
      {onSelect && <CardSelect label={name} selected={selected} onSelect={onSelect} />}
    </div>
  );
}
