import { ArrowLeft, Trash2 } from "lucide-react";
import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { formatAgentCount, translateError } from "../../i18n/translate";
import { isActive } from "../../hooks/agentRunState";
import { useLlmProviders } from "../../hooks/useLlmProviders";
import { useRoom } from "../../hooks/useRoom";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { Modal } from "../molecules/Modal";
import { ParticipantTile } from "../molecules/ParticipantTile";
import { RoomChat } from "./RoomChat";

type Props = {
  roomId: string;
  busy: boolean;
  deleteErrorMessage: string | null;
  onBack: () => void;
  onDelete: () => Promise<void>;
  onClearDeleteError: () => void;
};

export function RoomView({ roomId, busy, deleteErrorMessage, onBack, onDelete, onClearDeleteError }: Props) {
  const { t, locale } = useI18n();
  const { detail, errorKey, sendErrorKey, sending, sendIntervention, cancelRun } = useRoom(roomId);
  const providers = useLlmProviders();
  const [confirming, setConfirming] = useState(false);

  function closeConfirm() {
    onClearDeleteError();
    setConfirming(false);
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
  const { room, messages, runs } = detail;
  const thinking = new Set(Object.values(runs.runs).filter(isActive).map((run) => run.agent_id));

  return (
    <div className="flex min-h-[44rem] w-full flex-1 flex-col gap-5 lg:min-h-[34rem]">
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
      <header className="flex flex-col gap-2">
        {back}
        <h1 className="truncate text-2xl font-bold text-ink">{room.name}</h1>
      </header>
      <div className="grid min-h-0 flex-1 grid-rows-[minmax(0,12rem)_minmax(0,1fr)] gap-5 lg:grid-cols-[minmax(0,1fr)_24rem] lg:grid-rows-1">
        <section
          aria-label={t("rooms.participants")}
          tabIndex={0}
          className="min-h-0 overflow-y-auto rounded-2xl focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
        >
          <ul className="grid grid-cols-[repeat(auto-fill,minmax(10rem,1fr))] gap-4">
            <ParticipantTile name={t("rooms.you")} role={t("rooms.supervisor")} />
            {room.agents.map((agent) => (
              <ParticipantTile key={agent.id} name={agent.name} role={agent.role} thinking={thinking.has(agent.id)} />
            ))}
          </ul>
        </section>
        <RoomChat
          room={room}
          messages={messages}
          runs={runs}
          providers={providers}
          sending={sending}
          errorMessage={sendErrorKey ? translateError(locale, sendErrorKey) : null}
          onSend={sendIntervention}
          onCancel={cancelRun}
        />
      </div>
      <footer className="flex items-center justify-between gap-4 rounded-2xl border border-line bg-surface px-5 py-3">
        <p className="text-sm text-muted">{formatAgentCount(locale, room.agents.length)}</p>
        <Button type="button" variant="danger" size="compact" onClick={() => setConfirming(true)}>
          <Trash2 aria-hidden className="size-4" />
          {t("rooms.delete.action")}
        </Button>
      </footer>
    </div>
  );
}
