import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

export type Participant = { id: string; name: string; role: string };

export type Room = {
  id: string;
  name: string;
  updated_at: string;
  finished_at: string | null;
  agents: Participant[];
};

export type Message = {
  id: string;
  room_id: string;
  agent_id: string | null;
  body: string;
  created_at: string;
};

export type RoomList = { rooms: Room[]; featured_messages: Message[] };

export function useRooms() {
  const [list, setList] = useState<RoomList | null>(null);
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [actionErrorKey, setActionErrorKey] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const reload = useCallback(async () => {
    try {
      setList(await invoke<RoomList>("list_rooms"));
      setErrorKey(null);
    } catch (error) {
      setErrorKey(String(error));
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  async function createRoom(name: string, agentIds: string[]) {
    setBusy(true);
    setActionErrorKey(null);
    try {
      return await invoke<Room>("create_room", { name, agentIds });
    } catch (error) {
      setActionErrorKey(String(error));
      return null;
    } finally {
      setBusy(false);
    }
  }

  async function finishRoom(roomId: string) {
    setBusy(true);
    setActionErrorKey(null);
    try {
      setList(await invoke<RoomList>("finish_room", { roomId }));
      return true;
    } catch (error) {
      setActionErrorKey(String(error));
      return false;
    } finally {
      setBusy(false);
    }
  }

  const clearActionError = () => setActionErrorKey(null);

  return {
    list,
    errorKey,
    actionErrorKey,
    busy,
    reload,
    createRoom,
    finishRoom,
    clearActionError,
  };
}
