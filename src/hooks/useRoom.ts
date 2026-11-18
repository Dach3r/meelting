import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import {
  type AgentRun,
  type RunSnapshot,
  type RunState,
  type TokenChunk,
  applySnapshot,
  applyStatus,
  applyToken,
  emptyRunState,
  loadRunState,
} from "./agentRunState";
import type { Message, Room } from "./useRooms";

const recentMessages = 100;
const instructionTimeoutMs = 15000;
const instructionTries = 3;

type RoomDetail = { room: Room; messages: Message[]; runs: AgentRun[]; partials: RunSnapshot[] };

type InstructionSent = { message: Message; run: AgentRun; partial: RunSnapshot | null };

type Update = (state: { messages: Message[]; runs: RunState }) => { messages: Message[]; runs: RunState };

function withMessage(messages: Message[], message: Message) {
  return messages.some((known) => known.id === message.id) ? messages : [...messages, message].slice(-recentMessages);
}

function invokeWithTimeout<T>(command: string, args: Record<string, unknown>) {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("timeout")), instructionTimeoutMs);
    invoke<T>(command, args).then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (error) => {
        clearTimeout(timer);
        reject(error);
      },
    );
  });
}

export function useRoom(roomId: string) {
  const [room, setRoom] = useState<Room | null>(null);
  const [state, setState] = useState<{ messages: Message[]; runs: RunState }>({ messages: [], runs: emptyRunState });
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [sendErrorKey, setSendErrorKey] = useState<string | null>(null);
  const [sending, setSending] = useState(false);

  useEffect(() => {
    let current = true;
    let loaded = false;
    const waiting: Update[] = [];
    const receive = (change: Update) => {
      if (!current) return;
      if (loaded) setState(change);
      else waiting.push(change);
    };
    setRoom(null);
    setState({ messages: [], runs: emptyRunState });
    const subscriptions = Promise.all([
      listen<Message>("message:created", (event) => {
        const message = event.payload;
        if (message.room_id === roomId) receive((s) => ({ ...s, messages: withMessage(s.messages, message) }));
      }),
      listen<AgentRun>("agent_run:status", (event) => {
        const run = event.payload;
        if (run.room_id === roomId) receive((s) => ({ ...s, runs: applyStatus(s.runs, run) }));
      }),
      listen<TokenChunk>("agent_run:token", (event) => {
        const chunk = event.payload;
        if (chunk.room_id === roomId) receive((s) => ({ ...s, runs: applyToken(s.runs, chunk) }));
      }),
    ]);
    subscriptions
      .then(() => invoke<RoomDetail>("get_room", { roomId }))
      .then((detail) => {
        if (!current) return;
        loaded = true;
        setRoom(detail.room);
        setState(
          waiting.splice(0).reduce((s, change) => change(s), {
            messages: detail.messages,
            runs: loadRunState(detail.runs, detail.partials),
          }),
        );
      })
      .catch((error) => {
        if (current) setErrorKey(String(error));
      });
    return () => {
      current = false;
      subscriptions.then((stops) => stops.forEach((stop) => stop()));
    };
  }, [roomId]);

  async function sendMessage(body: string) {
    setSending(true);
    setSendErrorKey(null);
    try {
      const message = await invoke<Message>("send_message", { roomId, body });
      setState((s) => ({ ...s, messages: withMessage(s.messages, message) }));
      return true;
    } catch (error) {
      setSendErrorKey(String(error));
      return false;
    } finally {
      setSending(false);
    }
  }

  async function sendInstruction(agentId: string, body: string) {
    setSending(true);
    setSendErrorKey(null);
    const clientRequestId = crypto.randomUUID();
    try {
      for (let attempt = 1; attempt <= instructionTries; attempt += 1) {
        try {
          const sent = await invokeWithTimeout<InstructionSent>("send_instruction", { roomId, agentId, body, clientRequestId });
          setState((s) => {
            const runs = applyStatus(s.runs, sent.run);
            return {
              messages: withMessage(s.messages, sent.message),
              runs: sent.partial ? applySnapshot(runs, sent.partial) : runs,
            };
          });
          return true;
        } catch (error) {
          if (typeof error === "string") {
            setSendErrorKey(error);
            return false;
          }
        }
      }
      setSendErrorKey("unknown");
      return false;
    } finally {
      setSending(false);
    }
  }

  async function cancelRun(runId: string) {
    setSendErrorKey(null);
    try {
      await invoke("cancel_agent_run", { runId });
    } catch (error) {
      setSendErrorKey(String(error));
    }
  }

  const detail = room ? { room, messages: state.messages, runs: state.runs } : null;

  return { detail, errorKey, sendErrorKey, sending, sendMessage, sendInstruction, cancelRun };
}
