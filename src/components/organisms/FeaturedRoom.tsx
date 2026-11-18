import { ArrowRight } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { formatAgentCount, formatDateTime } from "../../i18n/translate";
import type { Message, Room } from "../../hooks/useRooms";
import { Button } from "../atoms/Button";
import { ParticipantTile } from "../molecules/ParticipantTile";
import { authorName } from "./authorName";

type Props = { room: Room; messages: Message[]; onOpen: () => void };

export function FeaturedRoom({ room, messages, onOpen }: Props) {
  const { t, locale } = useI18n();
  return (
    <section aria-label={room.name} className="flex flex-col gap-5 rounded-3xl border border-line bg-raised p-6">
      <div className="grid gap-1">
        <p className="text-xs font-semibold tracking-wider text-muted uppercase">{t("rooms.featured")}</p>
        <h2 className="truncate text-2xl font-bold text-ink">{room.name}</h2>
        <p className="text-sm text-muted">
          {t("rooms.lastActivity")} {formatDateTime(locale, room.updated_at)}
        </p>
      </div>
      <ul aria-label={t("rooms.participants")} className="grid grid-cols-[repeat(auto-fit,minmax(9rem,1fr))] gap-3">
        <ParticipantTile name={t("rooms.you")} role={t("rooms.supervisor")} />
        {room.agents.map((agent) => (
          <ParticipantTile key={agent.id} name={agent.name} role={agent.role} />
        ))}
      </ul>
      <div className="flex flex-col gap-3 rounded-2xl bg-surface p-4">
        <p className="text-xs font-semibold tracking-wider text-muted uppercase">{t("rooms.latestMessages")}</p>
        {messages.length === 0 ? (
          <p className="text-sm text-muted">{t("rooms.noMessages")}</p>
        ) : (
          <ul className="grid gap-3">
            {messages.map((message) => (
              <li key={message.id} className="grid gap-0.5 text-sm">
                <span className="font-semibold text-ink">{authorName(t, room, message)}</span>
                <span className="line-clamp-2 text-muted">{message.body}</span>
              </li>
            ))}
          </ul>
        )}
      </div>
      <div className="mt-auto flex items-center justify-between gap-3">
        <p className="text-sm font-medium text-muted">{formatAgentCount(locale, room.agents.length)}</p>
        <Button type="button" size="pill" onClick={onOpen}>
          {t("rooms.enter")}
          <ArrowRight aria-hidden className="size-4" />
        </Button>
      </div>
    </section>
  );
}
