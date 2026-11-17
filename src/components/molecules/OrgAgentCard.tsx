import { Avatar } from "../atoms/Avatar";

type Props = { seed: string; name: string; role: string };

export function OrgAgentCard({ seed, name, role }: Props) {
  return (
    <div className="flex h-12 w-39 items-center gap-2.5 rounded-full bg-raised p-1.5 pr-3">
      <Avatar seed={seed} label={name} className="size-9 rounded-full" />
      <div className="min-w-0 leading-tight">
        <p className="truncate font-semibold text-ink">{name}</p>
        <p className="truncate text-xs text-muted">{role}</p>
      </div>
    </div>
  );
}
