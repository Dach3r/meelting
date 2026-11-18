import { useId } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { formatAgentCount, formatDateTime } from "../../i18n/translate";
import type { Room } from "../../hooks/useRooms";
import { Avatar } from "../atoms/Avatar";
import { Button } from "../atoms/Button";

const SEATS = 3;

export function RoomCard({ room, onOpen }: { room: Room; onOpen: () => void }) {
  const { t, locale } = useI18n();
  const nameId = useId();
  const seated = room.agents.slice(0, SEATS);
  const extra = room.agents.length - seated.length;
  return (
    <li className="board-dots flex flex-col gap-3 rounded-[20px] border border-line bg-shell p-3">
      <ul aria-label={t("rooms.participants")} className="grid grid-cols-3 gap-1.5">
        {seated.map((agent) => (
          <li key={agent.id} className="relative grid h-26 place-items-center rounded-xl bg-surface">
            <Avatar seed={agent.name} label={agent.name} className="size-11 rounded-full text-lg" />
            <span className="absolute bottom-1.5 left-2 max-w-[90%] truncate text-[11px] text-muted">{agent.name}</span>
          </li>
        ))}
        {Array.from({ length: SEATS - seated.length }, (_, i) => (
          <li key={i} aria-hidden className="h-26 rounded-xl border border-dashed border-line" />
        ))}
      </ul>
      <div className="flex items-end justify-between gap-2 px-1.5 pb-1">
        <div className="grid min-w-0 gap-0.5">
          <h3 id={nameId} className="truncate text-base font-semibold text-ink">
            {room.name}
          </h3>
          <p className="text-xs text-muted">
            {formatDateTime(locale, room.updated_at)}
            {extra > 0 && ` · +${extra}`}
          </p>
          <p className="sr-only">
            {formatAgentCount(locale, room.agents.length)} · {t("rooms.lastActivity")} {formatDateTime(locale, room.updated_at)}
          </p>
        </div>
        <Button type="button" size="pill" aria-describedby={nameId} onClick={onOpen} className="shrink-0">
          {t("rooms.enter")}
        </Button>
      </div>
    </li>
  );
}
