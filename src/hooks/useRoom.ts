import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import type { Message, Room } from "./useRooms";

const recentMessages = 100;

type RoomDetail = { room: Room; messages: Message[] };

export function useRoom(roomId: string) {
  const [detail, setDetail] = useState<RoomDetail | null>(null);
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [sendErrorKey, setSendErrorKey] = useState<string | null>(null);
  const [sending, setSending] = useState(false);

  const append = useCallback((message: Message) => {
    setDetail((current) =>
      current && !current.messages.some((known) => known.id === message.id)
        ? { ...current, messages: [...current.messages, message].slice(-recentMessages) }
        : current,
    );
  }, []);

  useEffect(() => {
    let current = true;
    invoke<RoomDetail>("get_room", { roomId })
      .then((loaded) => {
        if (current) setDetail(loaded);
      })
      .catch((error) => {
        if (current) setErrorKey(String(error));
      });
    const unlisten = listen<Message>("message:created", (event) => {
      if (event.payload.room_id === roomId) append(event.payload);
    });
    return () => {
      current = false;
      unlisten.then((stop) => stop());
    };
  }, [roomId, append]);

  async function sendMessage(body: string) {
    setSending(true);
    setSendErrorKey(null);
    try {
      append(await invoke<Message>("send_message", { roomId, body }));
      return true;
    } catch (error) {
      setSendErrorKey(String(error));
      return false;
    } finally {
      setSending(false);
    }
  }

  return { detail, errorKey, sendErrorKey, sending, sendMessage };
}
