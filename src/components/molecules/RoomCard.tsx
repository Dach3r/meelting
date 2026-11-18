import { useI18n } from "../../i18n/I18nProvider";
import { formatAgentCount, formatDateTime } from "../../i18n/translate";
import type { Room } from "../../hooks/useRooms";
import { Avatar } from "../atoms/Avatar";

export function RoomCard({ room, onOpen }: { room: Room; onOpen: () => void }) {
  const { t, locale } = useI18n();
  return (
    <li>
      <button
        type="button"
        onClick={onOpen}
        className="flex w-full cursor-pointer flex-col gap-4 rounded-2xl border border-line bg-surface p-5 text-left transition-colors hover:bg-raised focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
      >
        <span className="flex items-center gap-3">
          <Avatar seed={room.name} label={room.name} className="size-10 rounded-xl text-base" />
          <span className="truncate font-semibold text-ink">{room.name}</span>
        </span>
        <span className="flex items-center justify-between gap-3 text-xs text-muted">
          <span>{formatAgentCount(locale, room.agents.length)}</span>
          <span>
            {t("rooms.lastActivity")} {formatDateTime(locale, room.updated_at)}
          </span>
        </span>
      </button>
    </li>
  );
}
