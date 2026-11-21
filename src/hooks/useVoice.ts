import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import type { Message } from "./useRooms";

type VoiceStatus = {
  available: boolean;
  model_ready: boolean;
  model_bytes: number;
};

type Playback = {
  room_id: string;
  message_id: string;
  agent_id: string | null;
  speaking: boolean;
};

export type VoicePhase =
  | "idle"
  | "downloading"
  | "starting"
  | "listening"
  | "transcribing";

export function useVoice(roomId: string, messages: Message[] | null) {
  const [status, setStatus] = useState<VoiceStatus | null>(null);
  const [phase, setPhase] = useState<VoicePhase>("idle");
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [speakingAgentId, setSpeakingAgentId] = useState<string | null>(null);
  const [aloud, setAloud] = useState(false);
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const known = useRef<Set<string> | null>(null);

  useEffect(() => {
    if (!messages) return;
    if (!known.current) {
      known.current = new Set(messages.map((message) => message.id));
      return;
    }
    for (const message of messages) {
      if (known.current.has(message.id)) continue;
      known.current.add(message.id);
      if (!aloud || !message.agent_id) continue;
      invoke("speak_message", { roomId, messageId: message.id }).catch(
        (error) => setErrorKey(String(error)),
      );
    }
  }, [messages, aloud, roomId]);

  useEffect(() => {
    let current = true;
    invoke<VoiceStatus>("get_voice_status")
      .then((loaded) => {
        if (current) setStatus(loaded);
      })
      .catch(() => {
        if (current)
          setStatus({ available: false, model_ready: false, model_bytes: 0 });
      });
    const subscription = listen<Playback>("voice:playback", (event) => {
      const playback = event.payload;
      if (playback.room_id !== roomId) return;
      setSpeakingAgentId((speaking) =>
        playback.speaking
          ? playback.agent_id
          : speaking === playback.agent_id
            ? null
            : speaking,
      );
    });
    return () => {
      current = false;
      subscription.then((stop) => stop());
      invoke("stop_speaking").catch(() => undefined);
      invoke("cancel_voice_capture").catch(() => undefined);
    };
  }, [roomId]);

  useEffect(() => {
    const subscription = listen<number>("voice:download_progress", (event) =>
      setDownloadedBytes(event.payload),
    );
    return () => {
      subscription.then((stop) => stop());
    };
  }, []);

  async function download() {
    setDownloadedBytes(0);
    setPhase("downloading");
    setErrorKey(null);
    try {
      setStatus(await invoke<VoiceStatus>("download_voice_model"));
    } catch (error) {
      setErrorKey(String(error));
    } finally {
      setPhase("idle");
    }
  }

  async function startListening() {
    setErrorKey(null);
    setPhase("starting");
    try {
      await invoke("start_voice_capture");
      setAloud(true);
      setPhase("listening");
    } catch (error) {
      setErrorKey(String(error));
      setPhase("idle");
    }
  }

  async function finish(): Promise<string | null> {
    setPhase("transcribing");
    try {
      return await invoke<string>("finish_voice_capture");
    } catch (error) {
      setErrorKey(String(error));
      return null;
    } finally {
      setPhase("idle");
    }
  }

  function cancel() {
    invoke("cancel_voice_capture").catch(() => undefined);
    setPhase("idle");
  }

  function stopSpeaking() {
    invoke("stop_speaking").catch(() => undefined);
    setSpeakingAgentId(null);
  }

  function toggleAloud() {
    if (aloud) stopSpeaking();
    setAloud(!aloud);
  }

  return {
    status,
    phase,
    errorKey,
    speakingAgentId,
    aloud,
    downloadedBytes,
    download,
    startListening,
    finish,
    cancel,
    stopSpeaking,
    toggleAloud,
    clearError: () => setErrorKey(null),
  };
}
