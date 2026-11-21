import {
  ArrowLeft,
  ListPlus,
  LoaderCircle,
  Mic,
  PhoneOff,
  RefreshCw,
  SendHorizontal,
  UserPlus,
  Volume2,
  VolumeOff,
  Wrench,
  X,
} from "lucide-react";
import { type ReactNode, useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import {
  type Locale,
  formatAgentCount,
  translateError,
  translateRunError,
  translateWith,
} from "../../i18n/translate";
import { type AgentRun, isActive, isFinal } from "../../hooks/agentRunState";
import {
  type AgentAvailability,
  useAgentAvailability,
} from "../../hooks/useAgentAvailability";
import type { LlmProvider } from "../../hooks/useLlmProviders";
import { useLlmProviders } from "../../hooks/useLlmProviders";
import { useAgentTools } from "../../hooks/useAgentTools";
import { useRoom } from "../../hooks/useRoom";
import { useTasks } from "../../hooks/useTasks";
import { useVoice } from "../../hooks/useVoice";
import type { Message, Participant } from "../../hooks/useRooms";
import type { Agent } from "../../hooks/useTeams";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { InviteAgentsForm } from "../molecules/InviteAgentsForm";
import { Modal } from "../molecules/Modal";
import { VoiceDownloadProgress } from "../molecules/VoiceDownloadProgress";
import {
  type ParticipantStatus,
  ParticipantTile,
} from "../molecules/ParticipantTile";
import { toolLabel } from "../molecules/ToolActivityList";
import type { VoiceDraft } from "../molecules/MessageComposer";
import { RoomChat } from "./RoomChat";
import { TaskChanges } from "./TaskChanges";
import { TaskDialog } from "./TaskDialog";

type Props = {
  roomId: string;
  agents: Agent[];
  busy: boolean;
  finishErrorMessage: string | null;
  onBack: () => void;
  onFinish: () => Promise<void>;
  onClearFinishError: () => void;
};

export function RoomView({
  roomId,
  agents,
  busy,
  finishErrorMessage,
  onBack,
  onFinish,
  onClearFinishError,
}: Props) {
  const { t, locale } = useI18n();
  const {
    detail,
    errorKey,
    sendErrorKey,
    sending,
    sendIntervention,
    cancelRun,
    stopRoom,
    rosterErrorKey,
    rosterBusy,
    addAgents,
    removeAgent,
    clearRosterError,
  } = useRoom(roomId);
  const providers = useLlmProviders();
  const {
    tasks,
    replace: replaceTask,
    prepareEnvironment,
    busy: tasksBusy,
    actionErrorKey: tasksErrorKey,
  } = useTasks();
  const agentTools = useAgentTools();
  const toolsOff =
    agentTools.tools?.available === true && !agentTools.tools.enabled;
  const roomTasks = (tasks ?? []).filter(
    (task) =>
      task.origin_room_id === roomId ||
      task.linked_rooms.some((link) => link.room_id === roomId),
  );
  const finishedRuns = detail
    ? Object.values(detail.runs.runs).filter(isFinal).length
    : 0;
  const { availability, checking, recheck } = useAgentAvailability(
    roomId,
    finishedRuns,
  );
  const [confirming, setConfirming] = useState(false);
  const [inviting, setInviting] = useState(false);
  const [pendingInvite, setPendingInvite] = useState<string[] | null>(null);
  const [leaving, setLeaving] = useState<Participant | null>(null);
  const voice = useVoice(roomId, detail?.messages ?? null);
  const [voiceDraft, setVoiceDraft] = useState<VoiceDraft | null>(null);
  const [offeringVoice, setOfferingVoice] = useState(false);
  const [changesTaskId, setChangesTaskId] = useState<string | null>(null);
  const [taskSource, setTaskSource] = useState<{
    message: Message | null;
  } | null>(null);

  async function sendSpoken() {
    const text = await voice.finish();
    if (text)
      setVoiceDraft((previous) => ({ id: (previous?.id ?? 0) + 1, text }));
  }

  async function downloadVoice() {
    await voice.download();
    setOfferingVoice(false);
  }

  function closeConfirm() {
    onClearFinishError();
    setConfirming(false);
  }

  function openInvite() {
    clearRosterError();
    setInviting(true);
  }

  function closeInvite() {
    clearRosterError();
    setInviting(false);
    setPendingInvite(null);
  }

  function closeLeave() {
    clearRosterError();
    setLeaving(null);
  }

  async function invite(agentIds: string[]) {
    if (!(await addAgents(agentIds))) return;
    closeInvite();
    recheck();
  }

  async function leave(agent: Participant) {
    if (await removeAgent(agent.id)) setLeaving(null);
  }

  const back = (
    <Button
      type="button"
      variant="ghost"
      size="compact"
      className="self-start"
      onClick={onBack}
    >
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
  const { room, formerAgents, messages, runs, proposal, toolCalls } = detail;
  const changesTask = roomTasks.find((task) => task.id === changesTaskId);
  const invitable = agents.filter(
    (agent) => !room.agents.some((present) => present.id === agent.id),
  );
  const activeRuns = Object.values(runs.runs).filter(isActive);

  function status(agentId: string): ParticipantStatus | undefined {
    if (voice.speakingAgentId === agentId)
      return { label: t("rooms.voice.speaking"), tone: "speaking" };
    const run = activeRuns.find((candidate) => candidate.agent_id === agentId);
    const tool = run
      ? [...toolCalls]
          .reverse()
          .find((call) => call.run_id === run.id && call.status === "running")
      : undefined;
    if (tool) return { label: toolLabel(locale, tool, true), tone: "active" };
    if (run) return { label: t(activityKey(run)), tone: "active" };
    const found = availability?.find((entry) => entry.agent_id === agentId);
    if (!found) return undefined;
    const problem = availabilityStatus(found, providers, locale, t);
    if (problem.tone === "problem" || !agentTools.tools?.enabled)
      return problem;
    const cloud =
      providers.find((entry) => entry.key === found.provider)?.location ===
      "cloud";
    if (!cloud && found.tools !== "unsupported") return problem;
    return {
      label: t("tools.none"),
      tone: "calm",
      detail: t(cloud ? "tools.cloud" : "tools.unsupported"),
    };
  }

  return (
    <div className="fixed inset-0 z-40 flex flex-col gap-3 bg-shell p-3 lg:flex-row">
      {offeringVoice && (
        <Modal
          title={t("rooms.voice.setupTitle")}
          onClose={() => setOfferingVoice(false)}
          bare
          narrow
        >
          <div className="relative grid justify-items-center gap-4 px-8 pt-10 pb-8 text-center">
            <button
              type="button"
              aria-label={t("teams.close")}
              onClick={() => setOfferingVoice(false)}
              className="absolute top-5 right-5 grid size-10 cursor-pointer place-items-center rounded-full bg-raised text-muted transition-colors hover:bg-hi hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
            >
              <X className="size-4" aria-hidden />
            </button>
            <span
              aria-hidden
              className="relative m-3 grid size-20 place-items-center rounded-full bg-raised text-ink ring-1 ring-line ring-offset-8 ring-offset-surface"
            >
              <Mic className="size-8" />
            </span>
            <span aria-hidden className="mb-2 flex h-5 items-center gap-1">
              {["h-2", "h-4", "h-5", "h-4", "h-2"].map((height, index) => (
                <span
                  key={index}
                  style={{ animationDelay: `${index * 0.15}s` }}
                  className={`w-1 animate-equalizer rounded-full bg-accent motion-reduce:animate-none ${height}`}
                />
              ))}
            </span>
            <h2 className="text-xl font-bold text-ink">
              {t("rooms.voice.setupTitle")}
            </h2>
            <p className="text-muted">
              {translateWith(locale, "rooms.voice.setupBody", {
                size: Math.round((voice.status?.model_bytes ?? 0) / 1_000_000),
              })}
            </p>
            {voice.errorKey && (
              <Alert message={translateError(locale, voice.errorKey)} />
            )}
            {voice.phase === "downloading" ? (
              <VoiceDownloadProgress
                downloaded={voice.downloadedBytes}
                total={voice.status?.model_bytes ?? 0}
              />
            ) : (
              <Button type="button" className="w-full" onClick={downloadVoice}>
                {t("rooms.voice.download")}
              </Button>
            )}
            <button
              type="button"
              onClick={() => setOfferingVoice(false)}
              className="cursor-pointer text-sm font-semibold text-muted underline-offset-4 transition-colors hover:text-ink hover:underline focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ink"
            >
              {t("rooms.voice.notNow")}
            </button>
          </div>
        </Modal>
      )}
      {changesTask && (
        <Modal
          title={translateWith(locale, "rooms.changesTitle", {
            title: changesTask.title,
          })}
          onClose={() => setChangesTaskId(null)}
        >
          <div className="p-6">
            <TaskChanges task={changesTask} />
          </div>
        </Modal>
      )}
      {confirming && (
        <Modal title={t("rooms.finish.title")} onClose={closeConfirm}>
          <div className="grid gap-5 p-6">
            <p className="text-muted">{t("rooms.finish.body")}</p>
            {finishErrorMessage && <Alert message={finishErrorMessage} />}
            <div className="flex justify-end gap-3">
              <Button
                type="button"
                variant="ghost"
                size="compact"
                onClick={closeConfirm}
              >
                {t("rooms.cancel")}
              </Button>
              <Button
                type="button"
                size="compact"
                disabled={busy}
                onClick={onFinish}
              >
                {t("rooms.finish.confirm")}
              </Button>
            </div>
          </div>
        </Modal>
      )}
      {inviting && !pendingInvite && (
        <Modal title={t("rooms.invite.title")} onClose={closeInvite}>
          <InviteAgentsForm
            agents={invitable}
            busy={rosterBusy}
            onSubmit={setPendingInvite}
            onCancel={closeInvite}
          />
        </Modal>
      )}
      {pendingInvite && (
        <Modal title={t("rooms.invite.confirmTitle")} onClose={closeInvite}>
          <div className="grid gap-5 p-6">
            <p className="text-muted">
              {translateWith(locale, "rooms.invite.confirmBody", {
                names: invitable
                  .filter((agent) => pendingInvite.includes(agent.id))
                  .map((agent) => agent.name)
                  .join(", "),
              })}
            </p>
            {rosterErrorKey && (
              <Alert message={translateError(locale, rosterErrorKey)} />
            )}
            <div className="flex justify-end gap-3">
              <Button
                type="button"
                variant="ghost"
                size="compact"
                onClick={() => setPendingInvite(null)}
              >
                {t("rooms.cancel")}
              </Button>
              <Button
                type="button"
                size="compact"
                disabled={rosterBusy}
                onClick={() => invite(pendingInvite)}
              >
                {t("rooms.invite.confirm")}
              </Button>
            </div>
          </div>
        </Modal>
      )}
      {taskSource && (
        <TaskDialog
          agents={agents}
          origin={{
            room_id: roomId,
            message_id: taskSource.message?.id ?? null,
          }}
          description={taskSource.message?.body}
          onSaved={(task) => {
            replaceTask(task);
            setTaskSource(null);
          }}
          onClose={() => setTaskSource(null)}
        />
      )}
      {leaving && (
        <Modal
          title={translateWith(locale, "rooms.leaveDialog.title", {
            name: leaving.name,
          })}
          onClose={closeLeave}
        >
          <div className="grid gap-5 p-6">
            <p className="text-muted">
              {translateWith(locale, "rooms.leaveDialog.body", {
                name: leaving.name,
              })}
            </p>
            {rosterErrorKey && (
              <Alert message={translateError(locale, rosterErrorKey)} />
            )}
            <div className="flex justify-end gap-3">
              <Button
                type="button"
                variant="ghost"
                size="compact"
                onClick={closeLeave}
              >
                {t("rooms.cancel")}
              </Button>
              <Button
                type="button"
                variant="destructive"
                size="compact"
                disabled={rosterBusy}
                onClick={() => leave(leaving)}
              >
                {t("rooms.leaveDialog.confirm")}
              </Button>
            </div>
          </div>
        </Modal>
      )}
      <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3">
        <header className="flex items-center gap-3">
          {back}
          <h1 className="min-w-0 flex-1 truncate text-xl font-bold text-ink">
            {room.name}
          </h1>
          <p className="shrink-0 rounded-full border border-line bg-surface px-3 py-1 text-sm text-muted">
            {formatAgentCount(locale, room.agents.length)}
          </p>
        </header>
        <section
          aria-label={t("rooms.participants")}
          className="relative flex min-h-0 flex-1 flex-col gap-3"
        >
          {rosterErrorKey && !inviting && !leaving && (
            <Alert message={translateError(locale, rosterErrorKey)} />
          )}
          <ul
            tabIndex={0}
            className={`grid min-h-0 flex-1 auto-rows-[minmax(10rem,1fr)] gap-3 overflow-y-auto rounded-2xl focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink ${columns(room.agents.length + 1)}`}
          >
            <ParticipantTile
              name={t("rooms.you")}
              role={t("rooms.supervisor")}
              status={
                voice.phase === "listening"
                  ? { label: t("rooms.voice.listening"), tone: "speaking" }
                  : voice.phase === "transcribing"
                    ? { label: t("rooms.voice.transcribing"), tone: "active" }
                    : undefined
              }
            />
            {room.agents.map((agent) => (
              <ParticipantTile
                key={agent.id}
                name={agent.name}
                role={agent.role}
                engine={`${providers.find((entry) => entry.key === agent.provider)?.name ?? agent.provider} · ${agent.model}`}
                status={status(agent.id)}
                removal={{
                  label: translateWith(locale, "rooms.leave", {
                    name: agent.name,
                  }),
                  disabled: rosterBusy,
                  onRemove: () => {
                    clearRosterError();
                    setLeaving(agent);
                  },
                }}
              />
            ))}
          </ul>
          {voice.errorKey && !offeringVoice && (
            <Alert message={translateError(locale, voice.errorKey)} />
          )}
          {toolsOff && room.agents.length > 0 && (
            <div className="flex shrink-0 flex-wrap items-center justify-between gap-3 rounded-2xl border border-line bg-shell px-4 py-2">
              <p className="text-sm text-muted">{t("tools.off")}</p>
              <Button
                type="button"
                size="compact"
                variant="ghost"
                disabled={agentTools.busy}
                onClick={() => agentTools.change(true)}
              >
                <Wrench aria-hidden className="size-4" />
                {t("tools.enable")}
              </Button>
            </div>
          )}
          {agentTools.errorKey && (
            <Alert message={translateError(locale, agentTools.errorKey)} />
          )}
          <div className="flex w-full shrink-0 items-center gap-2 rounded-2xl border border-line bg-shell p-2">
            {voice.status?.available &&
              (voice.phase === "listening" ? (
                <>
                  <ControlButton
                    label={t("rooms.voice.send")}
                    active
                    onClick={sendSpoken}
                  >
                    <SendHorizontal aria-hidden className="size-5" />
                  </ControlButton>
                  <ControlButton
                    label={t("rooms.voice.cancel")}
                    className=""
                    onClick={voice.cancel}
                  >
                    <X aria-hidden className="size-5" />
                  </ControlButton>
                </>
              ) : (
                <ControlButton
                  label={
                    voice.phase === "transcribing"
                      ? t("rooms.voice.transcribing")
                      : voice.phase === "starting"
                        ? t("rooms.voice.starting")
                        : t("rooms.voice.talk")
                  }
                  disabled={voice.phase !== "idle" || room.agents.length === 0}
                  onClick={() => {
                    voice.clearError();
                    if (voice.status?.model_ready) voice.startListening();
                    else setOfferingVoice(true);
                  }}
                >
                  {voice.phase === "transcribing" ||
                  voice.phase === "starting" ? (
                    <LoaderCircle
                      aria-hidden
                      className="size-5 animate-spin motion-reduce:animate-none"
                    />
                  ) : (
                    <Mic aria-hidden className="size-5" />
                  )}
                </ControlButton>
              ))}
            {voice.status?.available && (
              <ControlButton
                label={
                  voice.aloud
                    ? t("rooms.voice.aloudOn")
                    : t("rooms.voice.aloudOff")
                }
                pressed={voice.aloud}
                onClick={voice.toggleAloud}
              >
                {voice.aloud ? (
                  <Volume2 aria-hidden className="size-5" />
                ) : (
                  <VolumeOff aria-hidden className="size-5" />
                )}
              </ControlButton>
            )}
            <ControlButton
              label={t("rooms.invite.action")}
              onClick={openInvite}
            >
              <UserPlus aria-hidden className="size-5" />
            </ControlButton>
            <ControlButton
              label={t("tasks.fromRoom")}
              onClick={() => setTaskSource({ message: null })}
            >
              <ListPlus aria-hidden className="size-5" />
            </ControlButton>
            {room.agents.length > 0 && (
              <ControlButton
                label={t("rooms.availability.recheck")}
                disabled={checking}
                onClick={recheck}
              >
                <RefreshCw
                  aria-hidden
                  className={`size-5 ${checking ? "animate-spin" : ""}`}
                />
              </ControlButton>
            )}
            <ControlButton
              label={t("rooms.finish.action")}
              danger
              className="ml-auto"
              onClick={() => setConfirming(true)}
            >
              <PhoneOff aria-hidden className="size-5" />
            </ControlButton>
          </div>
        </section>
      </div>
      <div className="grid h-80 min-h-0 shrink-0 lg:h-auto lg:w-96">
        <RoomChat
          room={room}
          formerAgents={formerAgents}
          messages={messages}
          tasks={roomTasks}
          runs={runs}
          proposedAgentId={proposal}
          providers={providers}
          sending={sending}
          errorMessage={
            sendErrorKey ? translateError(locale, sendErrorKey) : null
          }
          onSend={sendIntervention}
          onCancel={cancelRun}
          onStop={stopRoom}
          onCreateTask={(message) => setTaskSource({ message })}
          toolCalls={toolCalls}
          voiceDraft={voiceDraft}
          onOpenChanges={setChangesTaskId}
          tasksBusy={tasksBusy}
          tasksErrorMessage={
            tasksErrorKey ? translateError(locale, tasksErrorKey) : null
          }
          onPrepareEnvironment={prepareEnvironment}
        />
      </div>
    </div>
  );
}

function columns(participants: number) {
  if (participants === 1) return "grid-cols-1";
  return participants <= 4
    ? "grid-cols-1 sm:grid-cols-2"
    : "grid-cols-1 sm:grid-cols-2 xl:grid-cols-3";
}

type ControlProps = {
  label: string;
  danger?: boolean;
  active?: boolean;
  pressed?: boolean;
  disabled?: boolean;
  className?: string;
  onClick: () => void;
  children: ReactNode;
};

function ControlButton({
  label,
  danger = false,
  active = false,
  pressed,
  disabled,
  className = "flex-1",
  onClick,
  children,
}: ControlProps) {
  return (
    <button
      type="button"
      aria-label={label}
      aria-pressed={pressed}
      title={label}
      disabled={disabled}
      onClick={onClick}
      className={`flex h-12 cursor-pointer items-center justify-center gap-2 rounded-xl border px-4 text-sm font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink disabled:cursor-not-allowed disabled:opacity-60 ${danger ? "border-error bg-error text-surface hover:bg-error/85" : active ? "border-accent bg-accent text-surface hover:bg-accent/85" : "border-line bg-raised text-ink hover:bg-hi"} ${className}`}
    >
      {children}
      <span className="hidden truncate sm:inline">{label}</span>
    </button>
  );
}

function activityKey(run: AgentRun) {
  if (run.status === "running") return "agentRun.thinking";
  return run.attempts > 0
    ? "rooms.availability.retrying"
    : "rooms.availability.queued";
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
      ? translateWith(locale, "rooms.availability.connectionDetail", {
          provider: name,
        })
      : translateRunError(
          locale,
          found.reason,
          name,
          provider?.location === "local",
        );
  return { label, tone: "problem", detail };
}
