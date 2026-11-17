import { Avatar } from "../atoms/Avatar";

type Props = { seed: string; name: string; role: string };

export function OrgAgentCard({ seed, name, role }: Props) {
  return (
    <div className="flex h-14 w-40 items-center gap-3 rounded-xl border border-line bg-raised px-3">
      <Avatar seed={seed} label={name} className="size-10 rounded-lg" />
      <div className="min-w-0">
        <p className="truncate font-semibold text-ink">{name}</p>
        <p className="truncate text-sm text-muted">{role}</p>
      </div>
    </div>
  );
}
