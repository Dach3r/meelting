import { Avatar } from "../atoms/Avatar";

type Props = { seed: string; name: string; role: string };

export function OrgAgentCard({ seed, name, role }: Props) {
  return (
    <div className="flex h-12 w-39 items-center gap-2.5 rounded-full bg-pill-hover p-1.5 pr-3 transition-colors hover:bg-hi">
      <Avatar seed={seed} label={name} className="size-9 rounded-full" />
      <div className="min-w-0 leading-tight">
        <p className="truncate text-sm font-semibold tracking-tight text-ink">{name}</p>
        <p className="truncate text-xs font-medium text-muted">{role}</p>
      </div>
    </div>
  );
}
