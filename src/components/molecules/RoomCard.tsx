import { useI18n } from "../../i18n/I18nProvider";
import { formatAgentCount, formatDateTime } from "../../i18n/translate";
import type { Room } from "../../hooks/useRooms";
import { Avatar } from "../atoms/Avatar";

const SEATS = 3;

export function RoomCard({ room, onOpen }: { room: Room; onOpen: () => void }) {
  const { t, locale } = useI18n();
  const seated = room.agents.slice(0, SEATS);
  const extra = room.agents.length - seated.length;
  return (
    <li>
      <button
        type="button"
        onClick={onOpen}
        className="flex h-full w-full cursor-pointer flex-col gap-4 rounded-2xl border border-line bg-raised p-4 text-left transition-colors hover:bg-hi focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
      >
        <span className="grid gap-0.5">
          <span className="truncate font-semibold text-ink">{room.name}</span>
          <span className="text-xs text-muted">{formatDateTime(locale, room.updated_at)}</span>
        </span>
        <span className="flex min-h-14 items-end gap-2">
          {seated.map((agent) => (
            <span key={agent.id} className="flex w-14 flex-col items-center gap-1">
              <Avatar seed={agent.name} label={agent.name} className="size-9 rounded-full text-sm" />
              <span className="w-full truncate text-center text-[11px] text-muted">{agent.name}</span>
            </span>
          ))}
          {extra > 0 && <span className="pb-5 text-xs font-medium text-muted">+{extra}</span>}
        </span>
        <span className="sr-only">
          {formatAgentCount(locale, room.agents.length)} · {t("rooms.lastActivity")} {formatDateTime(locale, room.updated_at)}
        </span>
      </button>
    </li>
  );
}
