import { ArrowLeft, RefreshCw, Trash2, UserPlus } from "lucide-react";
import { type ReactNode, useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { type Locale, formatAgentCount, translateError, translateRunError, translateWith } from "../../i18n/translate";
import { type AgentRun, isActive, isFinal } from "../../hooks/agentRunState";
import { type AgentAvailability, useAgentAvailability } from "../../hooks/useAgentAvailability";
import type { LlmProvider } from "../../hooks/useLlmProviders";
import { useLlmProviders } from "../../hooks/useLlmProviders";
import { useRoom } from "../../hooks/useRoom";
import type { Agent } from "../../hooks/useTeams";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { InviteAgentsForm } from "../molecules/InviteAgentsForm";
import { Modal } from "../molecules/Modal";
import { type ParticipantStatus, ParticipantTile } from "../molecules/ParticipantTile";
import { RoomChat } from "./RoomChat";

type Props = {
  roomId: string;
  agents: Agent[];
  busy: boolean;
  deleteErrorMessage: string | null;
  onBack: () => void;
  onDelete: () => Promise<void>;
  onClearDeleteError: () => void;
};

export function RoomView({ roomId, agents, busy, deleteErrorMessage, onBack, onDelete, onClearDeleteError }: Props) {
  const { t, locale } = useI18n();
  const { detail, errorKey, sendErrorKey, sending, sendIntervention, cancelRun, rosterErrorKey, rosterBusy, addAgents, removeAgent, clearRosterError } =
    useRoom(roomId);
  const providers = useLlmProviders();
  const finishedRuns = detail ? Object.values(detail.runs.runs).filter(isFinal).length : 0;
  const { availability, checking, recheck } = useAgentAvailability(roomId, finishedRuns);
  const [confirming, setConfirming] = useState(false);
  const [inviting, setInviting] = useState(false);

  function closeConfirm() {
    onClearDeleteError();
    setConfirming(false);
  }

  function openInvite() {
    clearRosterError();
    setInviting(true);
  }

  function closeInvite() {
    clearRosterError();
    setInviting(false);
  }

  async function invite(agentIds: string[]) {
    if (!(await addAgents(agentIds))) return;
    setInviting(false);
    recheck();
  }

  const back = (
    <Button type="button" variant="ghost" size="compact" className="self-start" onClick={onBack}>
      <ArrowLeft aria-hidden className="size-4" />
      {t("rooms.back")}
    </Button>
  );

  if (errorKey) {
    return (
      <div className="flex w-full max-w-4xl flex-col gap-4">
        {back}
        <Alert message={translateError(locale, errorKey)} />
      </div>
    );
  }

  if (!detail) return null;
  const { room, formerAgents, messages, runs, proposal } = detail;
  const invitable = agents.filter((agent) => !room.agents.some((present) => present.id === agent.id));
  const activeRuns = Object.values(runs.runs).filter(isActive);

  function status(agentId: string): ParticipantStatus | undefined {
    const run = activeRuns.find((candidate) => candidate.agent_id === agentId);
    if (run) return { label: t(activityKey(run)), tone: "active" };
    const found = availability?.find((entry) => entry.agent_id === agentId);
    return found && availabilityStatus(found, providers, locale, t);
  }

  return (
    <div className="fixed inset-0 z-40 flex flex-col gap-3 bg-shell p-3 lg:flex-row">
      {confirming && (
        <Modal title={t("rooms.delete.title")} onClose={closeConfirm}>
          <div className="grid gap-5 p-6">
            <p className="text-muted">{t("rooms.delete.body")}</p>
            {deleteErrorMessage && <Alert message={deleteErrorMessage} />}
            <div className="flex justify-end gap-3">
              <Button type="button" variant="ghost" size="compact" onClick={closeConfirm}>
                {t("rooms.cancel")}
              </Button>
              <Button type="button" size="compact" disabled={busy} onClick={onDelete}>
                {t("rooms.delete.confirm")}
              </Button>
            </div>
          </div>
        </Modal>
      )}
      {inviting && (
        <Modal title={t("rooms.invite.title")} onClose={closeInvite}>
          <InviteAgentsForm
            agents={invitable}
            busy={rosterBusy}
            errorMessage={rosterErrorKey ? translateError(locale, rosterErrorKey) : null}
            onSubmit={invite}
            onCancel={closeInvite}
          />
        </Modal>
      )}
      <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3">
        <header className="flex items-center gap-3">
          {back}
          <h1 className="min-w-0 flex-1 truncate text-xl font-bold text-ink">{room.name}</h1>
          <p className="shrink-0 rounded-full border border-line bg-surface px-3 py-1 text-sm text-muted">{formatAgentCount(locale, room.agents.length)}</p>
        </header>
        <section aria-label={t("rooms.participants")} className="relative flex min-h-0 flex-1 flex-col gap-3">
          {rosterErrorKey && !inviting && <Alert message={translateError(locale, rosterErrorKey)} />}
          <ul
            tabIndex={0}
            className={`grid min-h-0 flex-1 auto-rows-[minmax(10rem,1fr)] gap-3 overflow-y-auto rounded-2xl pb-20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink ${columns(room.agents.length + 1)}`}
          >
            <ParticipantTile name={t("rooms.you")} role={t("rooms.supervisor")} />
            {room.agents.map((agent) => (
              <ParticipantTile
                key={agent.id}
                name={agent.name}
                role={agent.role}
                status={status(agent.id)}
                removal={{
                  label: translateWith(locale, "rooms.leave", { name: agent.name }),
                  disabled: rosterBusy,
                  onRemove: () => removeAgent(agent.id),
                }}
              />
            ))}
          </ul>
          <div className="absolute bottom-3 left-1/2 flex -translate-x-1/2 items-center gap-2 rounded-2xl border border-line bg-shell/80 p-2 backdrop-blur">
            <ControlButton label={t("rooms.invite.action")} onClick={openInvite}>
              <UserPlus aria-hidden className="size-5" />
            </ControlButton>
            {room.agents.length > 0 && (
              <ControlButton label={t("rooms.availability.recheck")} disabled={checking} onClick={recheck}>
                <RefreshCw aria-hidden className={`size-5 ${checking ? "animate-spin" : ""}`} />
              </ControlButton>
            )}
            <ControlButton label={t("rooms.delete.action")} danger onClick={() => setConfirming(true)}>
              <Trash2 aria-hidden className="size-5" />
            </ControlButton>
          </div>
        </section>
      </div>
      <div className="grid h-80 min-h-0 shrink-0 lg:h-auto lg:w-96">
        <RoomChat
          room={room}
          formerAgents={formerAgents}
          messages={messages}
          runs={runs}
          proposedAgentId={proposal}
          providers={providers}
          sending={sending}
          errorMessage={sendErrorKey ? translateError(locale, sendErrorKey) : null}
          onSend={sendIntervention}
          onCancel={cancelRun}
        />
      </div>
    </div>
  );
}

function columns(participants: number) {
  if (participants === 1) return "grid-cols-1";
  return participants <= 4 ? "grid-cols-1 sm:grid-cols-2" : "grid-cols-1 sm:grid-cols-2 xl:grid-cols-3";
}

type ControlProps = { label: string; danger?: boolean; disabled?: boolean; onClick: () => void; children: ReactNode };

function ControlButton({ label, danger = false, disabled, onClick, children }: ControlProps) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={onClick}
      className={`grid size-12 cursor-pointer place-items-center rounded-xl border transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink disabled:cursor-not-allowed disabled:opacity-60 ${danger ? "border-error bg-error text-surface hover:bg-error/85" : "border-line bg-raised text-ink hover:bg-hi"}`}
    >
      {children}
    </button>
  );
}

function activityKey(run: AgentRun) {
  if (run.status === "running") return "agentRun.thinking";
  return run.attempts > 0 ? "rooms.availability.retrying" : "rooms.availability.queued";
}

function availabilityStatus(
  found: AgentAvailability,
  providers: LlmProvider[],
  locale: Locale,
  t: (key: string) => string,
): ParticipantStatus {
  const label = t(`rooms.availability.${found.availability}`);
  if (found.availability === "available") return { label, tone: "calm" };
  const provider = providers.find((entry) => entry.key === found.provider);
  const name = provider?.name ?? found.provider;
  const detail =
    found.availability === "needs_connection"
      ? translateWith(locale, "rooms.availability.connectionDetail", { provider: name })
      : translateRunError(locale, found.reason, name, provider?.location === "local");
  return { label, tone: "problem", detail };
}
