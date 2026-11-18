import { ArrowRight } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { formatAgentCount, formatDateTime } from "../../i18n/translate";
import type { Message, Room } from "../../hooks/useRooms";
import { Avatar } from "../atoms/Avatar";
import { Button } from "../atoms/Button";
import { authorName } from "./authorName";

type Props = { room: Room; messages: Message[]; onOpen: () => void };

export function FeaturedRoom({ room, messages, onOpen }: Props) {
  const { t, locale } = useI18n();
  return (
    <section aria-label={room.name} className="grid gap-6 rounded-3xl border border-line bg-surface p-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)]">
      <div className="flex flex-col gap-5">
        <div className="grid gap-1">
          <p className="text-xs font-semibold tracking-wider text-muted uppercase">{t("rooms.featured")}</p>
          <h2 className="truncate text-2xl font-bold text-ink">{room.name}</h2>
          <p className="text-sm text-muted">
            {t("rooms.lastActivity")} {formatDateTime(locale, room.updated_at)}
          </p>
        </div>
        <div className="grid gap-2">
          <p className="text-sm font-medium text-muted">{formatAgentCount(locale, room.agents.length)}</p>
          {room.agents.length > 0 && (
            <ul className="flex flex-wrap gap-2">
              {room.agents.map((agent) => (
                <li key={agent.id} className="flex max-w-full min-w-0 items-center gap-2 rounded-full border border-line bg-raised py-1 pr-3 pl-1 text-sm font-semibold text-ink">
                  <Avatar seed={agent.name} label={agent.name} className="size-6 shrink-0 rounded-full text-xs" />
                  <span className="truncate">{agent.name}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
        <Button type="button" size="pill" className="mt-auto self-start" onClick={onOpen}>
          {t("rooms.enter")}
          <ArrowRight aria-hidden className="size-4" />
        </Button>
      </div>
      <div className="flex flex-col gap-3 rounded-2xl bg-raised p-4">
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
    </section>
  );
}
