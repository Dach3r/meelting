import { SendHorizontal } from "lucide-react";
import { useId, useState, type FormEvent, type KeyboardEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { translateWith } from "../../i18n/translate";
import type { Participant } from "../../hooks/useRooms";
import { fieldClasses } from "../atoms/Field";

const bodyMaxChars = 4000;

type Props = {
  agents: Participant[];
  busyAgentIds: string[];
  sending: boolean;
  onSend: (body: string, agentId: string | null) => Promise<boolean>;
};

export function MessageComposer({ agents, busyAgentIds, sending, onSend }: Props) {
  const { t, locale } = useI18n();
  const limitId = useId();
  const busyId = useId();
  const [body, setBody] = useState("");
  const [agentId, setAgentId] = useState("");
  const length = Array.from(body.trim()).length;
  const tooLong = length > bodyMaxChars;
  const busyAgent = agents.find((agent) => agent.id === agentId && busyAgentIds.includes(agent.id));
  const disabled = length === 0 || tooLong || sending || busyAgent !== undefined;

  async function send() {
    if (disabled) return;
    if (await onSend(body, agentId || null)) setBody("");
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
    <form onSubmit={submit} className="grid gap-2">
      {tooLong && (
        <p id={limitId} className="text-sm font-medium text-error">
          {t("errors.invalid_message")}
        </p>
      )}
      {agents.length > 0 && (
        <label className="flex items-center gap-3 text-sm font-medium text-muted">
          {t("agentRun.askLabel")}
          <select
            value={agentId}
            onChange={(event) => setAgentId(event.target.value)}
            aria-describedby={busyAgent ? busyId : undefined}
            className={`${fieldClasses} min-h-11 flex-1 py-2`}
          >
            <option value="">{t("agentRun.nobody")}</option>
            {agents.map((agent) => (
              <option key={agent.id} value={agent.id}>
                {agent.name} · {agent.role}
              </option>
            ))}
          </select>
        </label>
      )}
      {busyAgent && (
        <p id={busyId} className="text-sm text-muted">
          {translateWith(locale, "agentRun.busy", { agent: busyAgent.name })}
        </p>
      )}
      <div className="flex items-end gap-2">
        <textarea
          aria-label={t("rooms.chat.message")}
          aria-invalid={tooLong}
          aria-describedby={tooLong ? limitId : undefined}
          value={body}
          onChange={(event) => setBody(event.target.value)}
          onKeyDown={onKeyDown}
          rows={1}
          placeholder={t("rooms.chat.placeholder")}
          className={`${fieldClasses} field-sizing-content max-h-40 min-h-14 resize-none py-3.5`}
        />
        <button
          type="submit"
          aria-label={t("rooms.chat.send")}
          disabled={disabled}
          className="grid size-14 shrink-0 cursor-pointer place-items-center rounded-xl bg-ink text-surface transition-colors enabled:hover:bg-ink/85 disabled:cursor-not-allowed disabled:opacity-60 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
        >
          <SendHorizontal aria-hidden className="size-5" />
        </button>
      </div>
    </form>
  );
}
