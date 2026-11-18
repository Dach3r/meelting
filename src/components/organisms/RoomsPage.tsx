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
import { RoomView } from "./RoomView";

export function RoomsPage({ agents }: { agents: Agent[] }) {
  const { t, locale } = useI18n();
  const { list, errorKey, actionErrorKey, busy, reload, createRoom, deleteRoom, clearActionError } = useRooms();
  const [creating, setCreating] = useState(false);
  const [openRoomId, setOpenRoomId] = useState<string | null>(null);
  const actionErrorMessage = actionErrorKey ? translateError(locale, actionErrorKey) : null;

  function openForm() {
    clearActionError();
    setCreating(true);
  }

  function closeForm() {
    clearActionError();
    setCreating(false);
  }

  async function create(name: string, agentIds: string[]) {
    const room = await createRoom(name, agentIds);
    if (!room) return;
    setCreating(false);
    setOpenRoomId(room.id);
  }

  function leaveRoom() {
    clearActionError();
    setOpenRoomId(null);
    reload();
  }

  async function removeRoom(roomId: string) {
    if (await deleteRoom(roomId)) setOpenRoomId(null);
  }

  if (openRoomId) {
    return (
      <RoomView
        key={openRoomId}
        roomId={openRoomId}
        agents={agents}
        busy={busy}
        deleteErrorMessage={actionErrorMessage}
        onBack={leaveRoom}
        onDelete={() => removeRoom(openRoomId)}
        onClearDeleteError={clearActionError}
      />
    );
  }

  const rooms = list?.rooms ?? [];

  return (
    <div className="flex w-full flex-col gap-6">
      {creating && (
        <Modal title={t("rooms.form.title")} onClose={closeForm}>
          <RoomForm agents={agents} busy={busy} errorMessage={actionErrorMessage} onSubmit={create} onCancel={closeForm} />
        </Modal>
      )}
      <header className="flex items-center justify-between gap-4">
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
            <RoomCard key={room.id} room={room} onOpen={() => setOpenRoomId(room.id)} />
          ))}
          <li className="flex">
            <button
              type="button"
              onClick={openForm}
              className="flex min-h-48 w-full cursor-pointer flex-col items-center justify-center gap-2.5 rounded-[20px] border border-dashed border-line text-sm text-muted transition-colors hover:bg-raised focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
            >
              <span className="grid size-11 place-items-center rounded-full bg-raised text-ink">
                <Plus aria-hidden className="size-5" />
              </span>
              {t("rooms.new")}
            </button>
          </li>
        </ul>
      )}
    </div>
  );
}
