import { useI18n } from "../../i18n/I18nProvider";
import { formatDateTime } from "../../i18n/translate";
import type { Message } from "../../hooks/useRooms";
import { Avatar } from "../atoms/Avatar";

export function ChatMessage({ message, author }: { message: Message; author: string }) {
  const { locale } = useI18n();
  return (
    <li className="flex min-w-0 gap-3">
      <Avatar seed={author} label={author} className="size-8 shrink-0 rounded-full text-sm" />
      <div className="grid min-w-0 flex-1 gap-1">
        <p className="flex min-w-0 items-baseline gap-2 text-sm">
          <span className="truncate font-semibold text-ink" title={author}>{author}</span>
          <time dateTime={message.created_at} className="shrink-0 text-xs whitespace-nowrap text-muted">
            {formatDateTime(locale, message.created_at)}
          </time>
        </p>
        <p className="text-sm whitespace-pre-wrap break-words text-ink select-text">{message.body}</p>
      </div>
    </li>
  );
}
