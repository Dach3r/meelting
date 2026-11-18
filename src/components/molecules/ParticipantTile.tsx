import { X } from "lucide-react";
import { Avatar } from "../atoms/Avatar";
import { roleRepeatsName } from "./agentLabel";

export type ParticipantStatus = { label: string; tone: "active" | "calm" | "problem"; detail?: string };

type Removal = { label: string; disabled: boolean; onRemove: () => void };

const toneClasses = { active: "text-accent", calm: "text-muted", problem: "text-error" };

export function ParticipantTile({ name, role, status, removal }: { name: string; role: string; status?: ParticipantStatus; removal?: Removal }) {
  return (
    <li className="group relative flex flex-col items-center gap-3 rounded-2xl border border-line bg-surface p-5 text-center">
      {removal && (
        <button
          type="button"
          aria-label={removal.label}
          title={removal.label}
          disabled={removal.disabled}
          onClick={removal.onRemove}
          className="absolute top-2 right-2 grid size-7 cursor-pointer place-items-center rounded-full text-muted opacity-0 transition-opacity group-hover:opacity-100 hover:bg-hi hover:text-ink focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-ink disabled:cursor-default disabled:opacity-40"
        >
          <X aria-hidden className="size-4" />
        </button>
      )}
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
