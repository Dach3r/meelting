import { Avatar } from "../atoms/Avatar";

export function ParticipantTile({ name, role }: { name: string; role: string }) {
  return (
    <li className="flex flex-col items-center gap-3 rounded-2xl border border-line bg-surface p-5 text-center">
      <Avatar seed={name} label={name} className="size-14 rounded-2xl text-xl" />
      <div className="grid min-w-0 gap-0.5">
        <p className="truncate font-semibold text-ink">{name}</p>
        <p className="truncate text-xs text-muted">{role}</p>
      </div>
    </li>
  );
}
