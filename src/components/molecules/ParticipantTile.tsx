import { Avatar } from "../atoms/Avatar";
import { roleRepeatsName } from "./agentLabel";

export type ParticipantStatus = { label: string; tone: "active" | "calm" | "problem"; detail?: string };

const toneClasses = { active: "text-accent", calm: "text-muted", problem: "text-error" };

export function ParticipantTile({ name, role, status }: { name: string; role: string; status?: ParticipantStatus }) {
  return (
    <li className="flex flex-col items-center gap-3 rounded-2xl border border-line bg-surface p-5 text-center">
      <Avatar seed={name} label={name} className={`size-14 rounded-2xl text-xl ${status?.tone === "active" ? "animate-pulse" : ""}`} />
      <div className="grid min-w-0 gap-0.5">
        <p className="truncate font-semibold text-ink">{name}</p>
        {!roleRepeatsName(name, role) && <p className="truncate text-xs text-muted">{role}</p>}
        {status && (
          <p title={status.detail} className={`text-xs font-medium ${toneClasses[status.tone]}`}>
            {status.label}
          </p>
        )}
        {status?.detail && <p className="sr-only">{status.detail}</p>}
      </div>
    </li>
  );
}
