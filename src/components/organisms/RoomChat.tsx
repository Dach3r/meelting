import { useEffect, useRef } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Message, Room } from "../../hooks/useRooms";
import { Alert } from "../molecules/Alert";
import { ChatMessage } from "../molecules/ChatMessage";
import { MessageComposer } from "../molecules/MessageComposer";
import { authorName } from "./authorName";

type Props = {
  room: Room;
  messages: Message[];
  sending: boolean;
  errorMessage: string | null;
  onSend: (body: string) => Promise<boolean>;
};

export function RoomChat({ room, messages, sending, errorMessage, onSend }: Props) {
  const { t } = useI18n();
  const end = useRef<HTMLLIElement>(null);
  const lastMessageId = messages[messages.length - 1]?.id;

  useEffect(() => {
    end.current?.scrollIntoView({ block: "end" });
  }, [lastMessageId]);

  return (
    <section aria-label={t("rooms.chat.title")} className="flex min-h-0 flex-col gap-4 rounded-2xl border border-line bg-surface p-5">
      <h2 className="font-semibold text-ink">{t("rooms.chat.title")}</h2>
      {messages.length === 0 ? (
        <p className="flex-1 text-sm text-muted">{t("rooms.noMessages")}</p>
      ) : (
        <ol
          aria-label={t("rooms.chat.transcript")}
          tabIndex={0}
          className="grid flex-1 content-start gap-4 overflow-y-auto overscroll-contain rounded-lg focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
        >
          {messages.map((message) => (
            <ChatMessage key={message.id} message={message} author={authorName(t, room, message)} />
          ))}
          <li ref={end} aria-hidden />
        </ol>
      )}
      {errorMessage && <Alert message={errorMessage} />}
      <MessageComposer sending={sending} onSend={onSend} />
    </section>
  );
}
