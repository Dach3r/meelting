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
  const seated = [{ id: "you", name: t("rooms.you") }, ...room.agents.slice(0, SEATS)];
  const extra = room.agents.length - (seated.length - 1);
  return (
    <li className="flex flex-col gap-5 rounded-[20px] border border-line bg-shell p-5">
      <div className="grid gap-0.5">
        <h3 id={nameId} className="truncate text-lg font-semibold text-ink">
          {room.name}
        </h3>
        <p className="text-xs text-muted">
          {t("rooms.lastActivity")} {formatDateTime(locale, room.updated_at)}
        </p>
      </div>
      <div className="grid min-h-14 content-center gap-2">
        <div className="flex items-center">
          {seated.map((participant) => (
            <Avatar key={participant.id} seed={participant.name} label={participant.name} className="-ml-2 size-11 rounded-full border-2 border-shell text-lg first:ml-0" />
          ))}
          {extra > 0 && <span className="ml-2 text-sm font-medium text-muted">+{extra}</span>}
        </div>
        <p className="truncate text-xs text-muted">
          {formatAgentCount(locale, room.agents.length)} · {seated.map((participant) => participant.name).join(", ")}
        </p>
      </div>
      <Button type="button" size="pill" aria-describedby={nameId} onClick={onOpen} className="mt-auto w-full">
        {t("rooms.enter")}
      </Button>
    </li>
  );
}
