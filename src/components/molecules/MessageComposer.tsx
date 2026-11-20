import { SendHorizontal, X } from "lucide-react";
import {
  useId,
  useMemo,
  useRef,
  useState,
  type FormEvent,
  type KeyboardEvent,
} from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { translateWith } from "../../i18n/translate";
import { useRecipientPreview } from "../../hooks/useRecipientPreview";
import type { Choice, Draft, Selection, SendResult } from "../../hooks/useRoom";
import type { Participant } from "../../hooks/useRooms";
import { Button } from "../atoms/Button";
import { fieldClasses } from "../atoms/Field";
import { MentionSuggestions, mentionOptionId } from "./MentionSuggestions";
import { agentLabel } from "./agentLabel";
import {
  type ChosenMention,
  activeMention,
  insertMention,
  keptMentionIds,
  matchingAgents,
} from "./mentionDraft";

const bodyMaxChars = 4000;

type Props = {
  roomId: string;
  agents: Participant[];
  proposedAgentId: string | null;
  busyAgentIds: string[];
  sending: boolean;
  onSend: (draft: Draft) => Promise<SendResult>;
};

export function MessageComposer({
  roomId,
  agents,
  proposedAgentId,
  busyAgentIds,
  sending,
  onSend,
}: Props) {
  const { t, locale } = useI18n();
  const limitId = useId();
  const statusId = useId();
  const choiceId = useId();
  const listId = useId();
  const field = useRef<HTMLTextAreaElement>(null);
  const [body, setBody] = useState("");
  const [caret, setCaret] = useState(0);
  const [chosen, setChosen] = useState<ChosenMention[]>([]);
  const [highlighted, setHighlighted] = useState(0);
  const [dismissedAt, setDismissedAt] = useState<number | null>(null);
  const [pending, setPending] = useState<{
    draft: Draft;
    choice: Choice;
  } | null>(null);
  const [dismissed, setDismissed] = useState<string | null>(null);

  const proposal = proposedAgentId !== dismissed ? proposedAgentId : null;
  const proposedAgent = agents.find((agent) => agent.id === proposal);
  const selection = useMemo<Selection>(
    () => ({ kind: "auto", agent_id: proposal }),
    [proposal],
  );
  const mentionIdsKey = keptMentionIds(body, chosen).join(",");
  const mentionIds = useMemo(
    () => (mentionIdsKey ? mentionIdsKey.split(",") : []),
    [mentionIdsKey],
  );
  const preview = useRecipientPreview(roomId, body, selection, mentionIds);

  const mention = activeMention(body, caret);
  const suggestions =
    mention && mention.start !== dismissedAt
      ? matchingAgents(agents, mention.query)
      : [];
  const open = suggestions.length > 0;
  const active = Math.min(highlighted, suggestions.length - 1);

  const length = Array.from(body.trim()).length;
  const tooLong = length > bodyMaxChars;
  const following =
    proposedAgent !== undefined && (!preview || preview.kind === "proposed");
  const targetId =
    preview && (preview.kind === "agent" || preview.kind === "proposed")
      ? preview.agent_id
      : following
        ? proposal
        : null;
  const target = agents.find((agent) => agent.id === targetId);
  const busy = target !== undefined && busyAgentIds.includes(target.id);
  const disabled = length === 0 || tooLong || sending || busy;

  function status() {
    if (busy && target)
      return translateWith(locale, "agentRun.busy", { agent: target.name });
    if (!preview) return null;
    if (preview.kind === "choose")
      return t(`agentRun.preview.${preview.reason}`);
    if (preview.kind === "round")
      return translateWith(locale, "agentRun.preview.round", {
        count: preview.agent_ids.length,
      });
    if (preview.kind === "proposed") return null;
    if (preview.kind === "agent" && target)
      return translateWith(locale, "agentRun.preview.agent", {
        agent: target.name,
      });
    return t("agentRun.preview.nobody");
  }

  function edit(text: string, position: number) {
    setBody(text);
    setCaret(position);
    setPending(null);
    setHighlighted(0);
  }

  function pick(agent: Participant) {
    if (!mention) return;
    const next = insertMention(body, mention, caret, agent.name);
    edit(next.body, next.caret);
    setChosen((previous) => [...previous, { id: agent.id, name: agent.name }]);
    requestAnimationFrame(() =>
      field.current?.setSelectionRange(next.caret, next.caret),
    );
  }

  async function deliver(draft: Draft) {
    const result = await onSend(draft);
    if (result === "failed") return;
    if (result === "sent") {
      edit("", 0);
      setChosen([]);
      setDismissed(null);
      return;
    }
    setPending({ draft, choice: result });
  }

  function send() {
    if (disabled) return;
    deliver({ body, selection, mentionIds });
  }

  function confirm(candidate: string) {
    if (!pending || sending) return;
    deliver({
      ...pending.draft,
      selection: { kind: "confirmed", agent_id: candidate },
    });
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    send();
  }

  function onKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.nativeEvent.isComposing) return;
    if (open && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      setHighlighted((active + step + suggestions.length) % suggestions.length);
      return;
    }
    if (
      open &&
      (event.key === "Enter" || event.key === "Tab") &&
      !event.shiftKey
    ) {
      event.preventDefault();
      pick(suggestions[active]);
      return;
    }
    if (open && event.key === "Escape") {
      event.preventDefault();
      setDismissedAt(mention?.start ?? null);
      return;
    }
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      send();
    }
  }

  const statusText = status();

  return (
    <form onSubmit={submit} className="grid gap-2">
      {tooLong && (
        <p id={limitId} className="text-sm font-medium text-error">
          {t("errors.invalid_message")}
        </p>
      )}
      {pending && (
        <div
          role="group"
          aria-labelledby={choiceId}
          className="grid gap-2 rounded-xl border border-line p-3"
        >
          <p id={choiceId} className="text-sm font-medium text-ink">
            {t(`agentRun.choose.${pending.choice.reason}`)}
          </p>
          <div className="flex flex-wrap gap-2">
            {pending.choice.candidates.map((candidate) => {
              const agent = agents.find((entry) => entry.id === candidate);
              return (
                agent && (
                  <Button
                    key={agent.id}
                    type="button"
                    variant="ghost"
                    size="compact"
                    disabled={sending}
                    onClick={() => confirm(agent.id)}
                  >
                    {agentLabel(agent)}
                  </Button>
                )
              );
            })}
            <Button
              type="button"
              variant="ghost"
              size="compact"
              onClick={() => setPending(null)}
            >
              {t("rooms.cancel")}
            </Button>
          </div>
        </div>
      )}
      {following && (
        <p className="flex items-center gap-2 text-sm text-muted">
          <span>
            {translateWith(locale, "agentRun.proposal", {
              agent: proposedAgent.name,
            })}{" "}
            <span className="rounded-full border border-line px-2 py-0.5 text-xs">
              {t("agentRun.proposed")}
            </span>
          </span>
          <button
            type="button"
            aria-label={translateWith(locale, "agentRun.dismissProposal", {
              agent: proposedAgent.name,
            })}
            onClick={() => setDismissed(proposedAgentId)}
            className="grid size-6 cursor-pointer place-items-center rounded-full hover:bg-ink/5 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
          >
            <X aria-hidden className="size-3.5" />
          </button>
        </p>
      )}
      <div className="relative flex items-end gap-2">
        {open && (
          <MentionSuggestions
            id={listId}
            label={t("agentRun.mentions")}
            agents={suggestions}
            active={active}
            onPick={pick}
          />
        )}
        <textarea
          ref={field}
          role="combobox"
          aria-label={t("rooms.chat.message")}
          aria-invalid={tooLong}
          aria-describedby={
            [tooLong ? limitId : null, statusText ? statusId : null]
              .filter(Boolean)
              .join(" ") || undefined
          }
          aria-autocomplete="list"
          aria-expanded={open}
          aria-controls={open ? listId : undefined}
          aria-activedescendant={
            open ? mentionOptionId(listId, active) : undefined
          }
          value={body}
          onChange={(event) =>
            edit(event.target.value, event.target.selectionStart)
          }
          onSelect={(event) => setCaret(event.currentTarget.selectionStart)}
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
      {statusText && (
        <p id={statusId} aria-live="polite" className="text-sm text-muted">
          {statusText}
        </p>
      )}
    </form>
  );
}
