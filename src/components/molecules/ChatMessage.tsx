import { ListPlus } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { formatDateTime, formatTime } from "../../i18n/translate";
import type { Message } from "../../hooks/useRooms";
import { Avatar } from "../atoms/Avatar";

export function ChatMessage({
  message,
  author,
  footer,
  onCreateTask,
}: {
  message: Message;
  author: string;
  footer?: string;
  onCreateTask: () => void;
}) {
  const { t, locale } = useI18n();
  return (
    <li className="group/message flex min-w-0 gap-3">
      <Avatar
        seed={author}
        label={author}
        className="size-8 shrink-0 rounded-full text-sm"
      />
      <div className="grid min-w-0 flex-1 gap-1">
        <p className="flex min-w-0 items-baseline gap-2 text-sm">
          <span className="truncate font-semibold text-ink" title={author}>
            {author}
          </span>
          <time
            dateTime={message.created_at}
            title={formatDateTime(locale, message.created_at)}
            className="shrink-0 text-xs whitespace-nowrap text-muted"
          >
            {formatTime(locale, message.created_at)}
          </time>
          <button
            type="button"
            aria-label={t("tasks.fromMessage")}
            title={t("tasks.fromMessage")}
            onClick={onCreateTask}
            className="ml-auto grid size-6 shrink-0 cursor-pointer place-items-center rounded-md text-muted opacity-0 transition-opacity group-hover/message:opacity-100 hover:bg-hi hover:text-ink focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-ink"
          >
            <ListPlus aria-hidden className="size-4" />
          </button>
        </p>
        <p className="text-sm whitespace-pre-wrap break-words text-ink">
          {message.body}
        </p>
        {footer && <p className="text-xs text-muted">{footer}</p>}
      </div>
    </li>
  );
}
