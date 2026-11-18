import { SendHorizontal } from "lucide-react";
import { useState, type FormEvent, type KeyboardEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { fieldClasses } from "../atoms/Field";

const bodyMaxChars = 4000;

type Props = {
  sending: boolean;
  onSend: (body: string) => Promise<boolean>;
};

export function MessageComposer({ sending, onSend }: Props) {
  const { t } = useI18n();
  const [body, setBody] = useState("");
  const blank = body.trim() === "";

  async function send() {
    if (blank || sending) return;
    if (await onSend(body)) setBody("");
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    send();
  }

  function onKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      send();
    }
  }

  return (
    <form onSubmit={submit} className="flex items-end gap-2">
      <textarea
        aria-label={t("rooms.chat.message")}
        value={body}
        onChange={(event) => setBody(event.target.value)}
        onKeyDown={onKeyDown}
        maxLength={bodyMaxChars}
        rows={1}
        placeholder={t("rooms.chat.placeholder")}
        className={`${fieldClasses} field-sizing-content max-h-40 min-h-14 resize-none py-3.5`}
      />
      <button
        type="submit"
        aria-label={t("rooms.chat.send")}
        disabled={blank || sending}
        className="grid size-14 shrink-0 cursor-pointer place-items-center rounded-xl bg-ink text-surface transition-colors enabled:hover:bg-ink/85 disabled:cursor-not-allowed disabled:opacity-60 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
      >
        <SendHorizontal aria-hidden className="size-5" />
      </button>
    </form>
  );
}
