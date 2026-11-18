import { ArrowRight } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { formatAgentCount, formatDateTime } from "../../i18n/translate";
import type { Message, Room } from "../../hooks/useRooms";
import { Avatar } from "../atoms/Avatar";
import { Button } from "../atoms/Button";
import { ParticipantTile } from "../molecules/ParticipantTile";
import { authorName } from "./authorName";

type Props = { room: Room; messages: Message[]; onOpen: () => void };

export function FeaturedRoom({ room, messages, onOpen }: Props) {
  const { t, locale } = useI18n();
  return (
    <section aria-label={room.name} className="flex flex-col gap-6 rounded-3xl border border-line bg-shell p-7">
      <div className="flex items-start justify-between gap-4">
        <div className="grid min-w-0 gap-1">
          <p className="text-xs font-semibold tracking-wider text-muted uppercase">{t("rooms.featured")}</p>
          <h2 className="truncate text-3xl font-bold tracking-tight text-ink">{room.name}</h2>
        </div>
        <div className="grid shrink-0 justify-items-end gap-0.5 pt-1 text-sm text-muted">
          <span className="font-medium text-ink">{formatAgentCount(locale, room.agents.length)}</span>
          <span>
            {t("rooms.lastActivity")} {formatDateTime(locale, room.updated_at)}
          </span>
        </div>
      </div>
      <ul aria-label={t("rooms.participants")} className="grid grid-cols-[repeat(auto-fit,minmax(9rem,1fr))] gap-3 *:justify-center">
        <ParticipantTile name={t("rooms.you")} role={t("rooms.supervisor")} />
        {room.agents.map((agent) => (
          <ParticipantTile key={agent.id} name={agent.name} role={agent.role} />
        ))}
      </ul>
      <div className="grid gap-1 rounded-2xl bg-surface px-5 py-4">
        <p className="pb-1 text-xs font-semibold tracking-wider text-muted uppercase">{t("rooms.latestMessages")}</p>
        {messages.length === 0 ? (
          <p className="text-sm text-muted">{t("rooms.noMessages")}</p>
        ) : (
          <ul className="divide-y divide-line/60">
            {messages.map((message) => {
              const author = authorName(t, room, message);
              return (
                <li key={message.id} className="flex items-start gap-3 py-3 text-sm first:pt-1 last:pb-0">
                  <Avatar seed={author} label={author} className="mt-0.5 size-7 rounded-full text-xs" />
                  <div className="grid min-w-0 gap-0.5">
                    <span className="font-semibold text-ink">{author}</span>
                    <span className="line-clamp-2 text-muted">{message.body}</span>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>
      <Button type="button" size="pill" className="mt-auto w-full justify-center" onClick={onOpen}>
        {t("rooms.enter")}
        <ArrowRight aria-hidden className="size-4" />
      </Button>
    </section>
  );
}
