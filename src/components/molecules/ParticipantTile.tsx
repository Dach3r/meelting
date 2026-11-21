import { X } from "lucide-react";
import type { CSSProperties } from "react";
import { Avatar, hueOf } from "../atoms/Avatar";
import { roleRepeatsName } from "./agentLabel";

export type ParticipantStatus = {
  label: string;
  tone: "speaking" | "active" | "calm" | "problem";
  detail?: string;
};

type Removal = { label: string; disabled: boolean; onRemove: () => void };

const toneClasses = {
  speaking: "text-accent",
  active: "text-accent",
  calm: "text-muted",
  problem: "text-error",
};

export function ParticipantTile({
  name,
  role,
  engine,
  status,
  removal,
}: {
  name: string;
  role: string;
  engine?: string;
  status?: ParticipantStatus;
  removal?: Removal;
}) {
  const speaking = status?.tone === "speaking";
  const active = speaking || status?.tone === "active";
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
      <Avatar
        seed={name}
        label={name}
        className={`size-24 rounded-3xl text-4xl ${active && !speaking ? "animate-pulse" : ""}`}
      />
      {speaking && (
        <span
          aria-hidden
          className="absolute bottom-14 flex h-5 items-end gap-1"
        >
          {["h-2", "h-5", "h-3", "h-4"].map((height, index) => (
            <span
              key={index}
              style={{ animationDelay: `${index * 0.15}s` }}
              className={`w-1 animate-equalizer rounded-full bg-accent motion-reduce:animate-none ${height}`}
            />
          ))}
        </span>
      )}
      {status && (
        <p
          title={status.detail}
          className={`absolute top-3 right-3 rounded-full bg-shell/80 px-2.5 py-1 text-xs font-medium backdrop-blur ${toneClasses[status.tone]}`}
        >
          {status.label}
        </p>
      )}
      {status?.detail && <p className="sr-only">{status.detail}</p>}
      <div className="absolute bottom-3 left-3 flex max-w-[calc(100%-1.5rem)] items-baseline gap-2 rounded-lg bg-shell/80 px-2.5 py-1 backdrop-blur">
        <p className="truncate text-sm font-semibold text-ink">{name}</p>
        {!roleRepeatsName(name, role) && (
          <p className="truncate text-xs text-muted">{role}</p>
        )}
      </div>
      {engine && (
        <p className="absolute right-3 bottom-3 max-w-[40%] truncate rounded-lg bg-shell/80 px-2.5 py-1 text-xs text-muted backdrop-blur">
          {engine}
        </p>
      )}
    </li>
  );
}
