import { Plus } from "lucide-react";
import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { translateError } from "../../i18n/translate";
import { useRooms } from "../../hooks/useRooms";
import type { Agent } from "../../hooks/useTeams";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { Modal } from "../molecules/Modal";
import { RoomCard } from "../molecules/RoomCard";
import { RoomForm } from "../molecules/RoomForm";
import { TransitionOverlay, transitionMs } from "../molecules/TransitionOverlay";
import { RoomView } from "./RoomView";

export function RoomsPage({ agents, loadingEnabled }: { agents: Agent[]; loadingEnabled: boolean }) {
  const { t, locale } = useI18n();
  const { list, errorKey, actionErrorKey, busy, reload, createRoom, finishRoom, clearActionError } = useRooms();
  const [creating, setCreating] = useState(false);
  const [openRoom, setOpenRoom] = useState<{ id: string; name: string } | null>(null);
  const [transition, setTransition] = useState<{ label: string; name: string } | null>(null);
  const actionErrorMessage = actionErrorKey ? translateError(locale, actionErrorKey) : null;

  function openForm() {
    clearActionError();
    setCreating(true);
  }

  function closeForm() {
    clearActionError();
    setCreating(false);
  }

  async function withTransition(label: string, name: string, apply: () => void) {
    if (!loadingEnabled) return apply();
    setTransition({ label, name });
    await new Promise((resolve) => setTimeout(resolve, transitionMs));
    apply();
    setTransition(null);
  }

  function enterRoom(room: { id: string; name: string }) {
    return withTransition(t("rooms.entering"), room.name, () => setOpenRoom(room));
  }

  async function create(name: string, agentIds: string[]) {
    const room = await createRoom(name, agentIds);
    if (!room) return;
    setCreating(false);
    await enterRoom(room);
  }

  function leaveRoom(room: { id: string; name: string }) {
    return withTransition(t("rooms.leaving"), room.name, () => setOpenRoom(null));
  }

  async function endRoom(room: { id: string; name: string }) {
    if (await finishRoom(room.id)) await leaveRoom(room);
  }

  const rooms = list?.rooms ?? [];

  const overlay = transition && <TransitionOverlay label={transition.label} name={transition.name} />;

  if (openRoom) {
    return (
      <>
        <RoomView
          key={openRoom.id}
          roomId={openRoom.id}
          agents={agents}
          busy={busy}
          finishErrorMessage={actionErrorMessage}
          onBack={() => {
            clearActionError();
            reload();
            return leaveRoom(openRoom);
          }}
          onFinish={() => endRoom(openRoom)}
          onClearFinishError={clearActionError}
        />
        {overlay}
      </>
    );
  }

  return (
    <div className="flex w-full flex-col gap-6">
      {overlay}
      {creating && (
        <Modal title={t("rooms.form.title")} onClose={closeForm}>
          <RoomForm agents={agents} busy={busy} errorMessage={actionErrorMessage} onSubmit={create} onCancel={closeForm} />
        </Modal>
      )}
      <header className="-mx-8 -mt-8 flex items-center justify-between gap-4 border-b border-line bg-linear-to-b from-shell to-surface px-8 py-5">
        <h1 className="text-2xl font-bold text-ink">{t("rooms.title")}</h1>
        <Button type="button" size="pill" onClick={openForm}>
          <Plus aria-hidden className="size-4" />
          {t("rooms.new")}
        </Button>
      </header>
      {errorKey && <Alert message={translateError(locale, errorKey)} />}
      {list && rooms.length === 0 && (
        <div className="grid gap-1 py-16 text-center">
          <h2 className="text-xl font-bold">{t("rooms.empty.title")}</h2>
          <p className="text-muted">{t("rooms.empty.body")}</p>
        </div>
      )}
      {list && rooms.length > 0 && (
        <ul aria-label={t("rooms.title")} className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
          {rooms.map((room) => (
            <RoomCard key={room.id} room={room} onOpen={() => enterRoom(room)} />
          ))}
        </ul>
      )}
    </div>
  );
}
