import { Fragment, useEffect, useRef, useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { translateWith } from "../../i18n/translate";
import {
  type AgentRun,
  type RunState,
  isActive,
  liveText,
} from "../../hooks/agentRunState";
import type { LlmProvider } from "../../hooks/useLlmProviders";
import type { Draft, SendResult } from "../../hooks/useRoom";
import type { Message, Participant, Room } from "../../hooks/useRooms";
import type { Task } from "../../hooks/useTasks";
import { Alert } from "../molecules/Alert";
import { AgentRunStatus } from "../molecules/AgentRunStatus";
import { ChatMessage } from "../molecules/ChatMessage";
import { MessageComposer } from "../molecules/MessageComposer";
import { RoomTasks } from "../molecules/RoomTasks";
import { SegmentedControl } from "../molecules/SegmentedControl";
import { authorName } from "./authorName";

type Props = {
  room: Room;
  formerAgents: Participant[];
  messages: Message[];
  tasks: Task[];
  runs: RunState;
  proposedAgentId: string | null;
  providers: LlmProvider[];
  sending: boolean;
  errorMessage: string | null;
  onSend: (draft: Draft) => Promise<SendResult>;
  onCancel: (runId: string) => void;
  onCreateTask: (message: Message) => void;
};

export function RoomChat({
  room,
  formerAgents,
  messages,
  tasks,
  runs,
  proposedAgentId,
  providers,
  sending,
  errorMessage,
  onSend,
  onCancel,
  onCreateTask,
}: Props) {
  const { t, locale } = useI18n();
  const [tab, setTab] = useState<"chat" | "tasks">("chat");
  const end = useRef<HTMLLIElement>(null);
  const allRuns = Object.values(runs.runs);
  const lastMessageId = messages[messages.length - 1]?.id;
  const lastLive = Object.values(runs.live).reduce(
    (total, live) => total + live.text.length,
    0,
  );
  const busyAgentIds = allRuns.filter(isActive).map((run) => run.agent_id);
  const knownAgents = [...room.agents, ...formerAgents];

  useEffect(() => {
    end.current?.scrollIntoView({ block: "end" });
  }, [lastMessageId, lastLive]);

  const provider = (run: AgentRun) =>
    providers.find((entry) => entry.key === run.provider);

  function footer(message: Message) {
    const run = allRuns.find(
      (candidate) => candidate.response_message_id === message.id,
    );
    if (!run) return undefined;
    const parts = [`${provider(run)?.name ?? run.provider} · ${run.model}`];
    if (run.input_tokens !== null && run.output_tokens !== null) {
      parts.push(
        translateWith(locale, "agentRun.tokens", {
          input: run.input_tokens,
          output: run.output_tokens,
        }),
      );
    }
    return parts.join(" · ");
  }

  return (
    <section
      aria-label={t("rooms.chat.title")}
      className="flex min-h-0 flex-col gap-4 rounded-2xl border border-line bg-surface p-5"
    >
      <h2 className="sr-only">{t("rooms.chat.title")}</h2>
      <SegmentedControl
        label={t("rooms.chat.tabs")}
        value={tab}
        onChange={setTab}
        fullWidth
        options={[
          { value: "chat", label: t("rooms.chat.tabChat") },
          { value: "tasks", label: t("rooms.chat.tabTasks") },
        ]}
      />
      {tab === "tasks" ? (
        <RoomTasks tasks={tasks} />
      ) : messages.length === 0 ? (
        <p className="flex-1 text-sm text-muted">{t("rooms.noMessages")}</p>
      ) : (
        <ol
          aria-label={t("rooms.chat.transcript")}
          tabIndex={0}
          className="grid flex-1 content-start gap-4 overflow-y-auto overscroll-contain rounded-lg focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
        >
          {messages.map((message) => {
            const run = allRuns.find(
              (candidate) => candidate.request_message_id === message.id,
            );
            return (
              <Fragment key={message.id}>
                <ChatMessage
                  message={message}
                  author={authorName(t, knownAgents, message)}
                  footer={footer(message)}
                  onCreateTask={() => onCreateTask(message)}
                />
                {run && (
                  <AgentRunStatus
                    run={run}
                    agentName={
                      knownAgents.find((agent) => agent.id === run.agent_id)
                        ?.name ?? ""
                    }
                    providerName={provider(run)?.name ?? run.provider}
                    local={provider(run)?.location === "local"}
                    liveText={liveText(runs, run)}
                    onCancel={() => onCancel(run.id)}
                  />
                )}
              </Fragment>
            );
          })}
          <li ref={end} aria-hidden />
        </ol>
      )}
      {tab === "chat" && errorMessage && <Alert message={errorMessage} />}
      {tab === "chat" && (
        <MessageComposer
          roomId={room.id}
          agents={room.agents}
          proposedAgentId={proposedAgentId}
          busyAgentIds={busyAgentIds}
          sending={sending}
          onSend={onSend}
        />
      )}
    </section>
  );
}
