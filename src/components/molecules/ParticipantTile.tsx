import { X } from "lucide-react";
import type { CSSProperties } from "react";
import { Avatar, hueOf } from "../atoms/Avatar";
import { roleRepeatsName } from "./agentLabel";

export type ParticipantStatus = { label: string; tone: "active" | "calm" | "problem"; detail?: string };

type Removal = { label: string; disabled: boolean; onRemove: () => void };

const toneClasses = { active: "text-accent", calm: "text-muted", problem: "text-error" };

export function ParticipantTile({ name, role, status, removal }: { name: string; role: string; status?: ParticipantStatus; removal?: Removal }) {
  const active = status?.tone === "active";
  return (
    <li
      style={{ "--tile-hue": hueOf(name) } as CSSProperties}
      className={`group relative grid min-h-0 place-items-center overflow-hidden rounded-2xl border bg-linear-to-br from-[hsl(var(--tile-hue)_30%_22%)] to-surface ${active ? "border-accent ring-2 ring-accent" : "border-line"} ${status?.tone === "problem" ? "opacity-70" : ""}`}
    >
      {removal && (
        <button
          type="button"
          aria-label={removal.label}
          title={removal.label}
          disabled={removal.disabled}
          onClick={removal.onRemove}
          className="absolute top-3 left-3 grid size-8 cursor-pointer place-items-center rounded-full bg-shell/80 text-muted opacity-0 backdrop-blur transition-opacity group-hover:opacity-100 hover:text-ink focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-ink disabled:cursor-default disabled:opacity-40"
        >
          <X aria-hidden className="size-4" />
        </button>
      )}
      <Avatar seed={name} label={name} className={`size-24 rounded-3xl text-4xl ${active ? "animate-pulse" : ""}`} />
      {status && (
        <p title={status.detail} className={`absolute top-3 right-3 rounded-full bg-shell/80 px-2.5 py-1 text-xs font-medium backdrop-blur ${toneClasses[status.tone]}`}>
          {status.label}
        </p>
      )}
      {status?.detail && <p className="sr-only">{status.detail}</p>}
      <div className="absolute bottom-3 left-3 flex max-w-[calc(100%-1.5rem)] items-baseline gap-2 rounded-lg bg-shell/80 px-2.5 py-1 backdrop-blur">
        <p className="truncate text-sm font-semibold text-ink">{name}</p>
        {!roleRepeatsName(name, role) && <p className="truncate text-xs text-muted">{role}</p>}
      </div>
    </li>
  );
}
