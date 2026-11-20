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
import type { Message, Participant, Room } from "./useRooms";

const recentMessages = 100;
const interventionTimeoutMs = 15000;
const interventionTries = 3;

type RoomDetail = {
  room: Room;
  former_agents: Participant[];
  messages: Message[];
  runs: AgentRun[];
  partials: RunSnapshot[];
  proposed_agent_id: string | null;
};

export type Selection =
  | { kind: "auto"; agent_id: string | null }
  | { kind: "confirmed"; agent_id: string };

export type Draft = {
  body: string;
  selection: Selection;
  mentionIds: string[];
};

export type Choice = {
  reason: "ambiguous" | "several" | "conflict";
  candidates: string[];
};

export type SendResult = "sent" | "failed" | Choice;

type InterventionSent =
  | {
      outcome: "sent";
      message: Message;
      run: AgentRun | null;
      partial: RunSnapshot | null;
    }
  | ({ outcome: "choose" } & Choice);

type Roster = { agents: Participant[]; former_agents: Participant[] };

type RoomState = {
  messages: Message[];
  runs: RunState;
  proposal: string | null;
};

type Update = (state: RoomState) => RoomState;

function withMessage(messages: Message[], message: Message) {
  return messages.some((known) => known.id === message.id)
    ? messages
    : [...messages, message].slice(-recentMessages);
}

function withStatus(state: RoomState, run: AgentRun): RoomState {
  const runs = applyStatus(state.runs, run);
  const replied = run.status === "succeeded" && runs !== state.runs;
  return { ...state, runs, proposal: replied ? run.agent_id : state.proposal };
}

function invokeWithTimeout<T>(command: string, args: Record<string, unknown>) {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new Error("timeout")),
      interventionTimeoutMs,
    );
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
  const [state, setState] = useState<RoomState>({
    messages: [],
    runs: emptyRunState,
    proposal: null,
  });
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const [sendErrorKey, setSendErrorKey] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const [formerAgents, setFormerAgents] = useState<Participant[]>([]);
  const [rosterErrorKey, setRosterErrorKey] = useState<string | null>(null);
  const [rosterBusy, setRosterBusy] = useState(false);

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
    setState({ messages: [], runs: emptyRunState, proposal: null });
    const subscriptions = Promise.all([
      listen<Message>("message:created", (event) => {
        const message = event.payload;
        if (message.room_id === roomId)
          receive((s) => ({
            ...s,
            messages: withMessage(s.messages, message),
          }));
      }),
      listen<AgentRun>("agent_run:status", (event) => {
        const run = event.payload;
        if (run.room_id === roomId) receive((s) => withStatus(s, run));
      }),
      listen<TokenChunk>("agent_run:token", (event) => {
        const chunk = event.payload;
        if (chunk.room_id === roomId)
          receive((s) => ({ ...s, runs: applyToken(s.runs, chunk) }));
      }),
    ]);
    subscriptions
      .then(() => invoke<RoomDetail>("get_room", { roomId }))
      .then((detail) => {
        if (!current) return;
        loaded = true;
        setRoom(detail.room);
        setFormerAgents(detail.former_agents);
        setState(
          waiting.splice(0).reduce((s, change) => change(s), {
            messages: detail.messages,
            runs: loadRunState(detail.runs, detail.partials),
            proposal: detail.proposed_agent_id,
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

  async function sendIntervention({
    body,
    selection,
    mentionIds,
  }: Draft): Promise<SendResult> {
    setSending(true);
    setSendErrorKey(null);
    const clientRequestId = crypto.randomUUID();
    try {
      for (let attempt = 1; attempt <= interventionTries; attempt += 1) {
        try {
          const sent = await invokeWithTimeout<InterventionSent>(
            "send_intervention",
            {
              roomId,
              body,
              selection,
              mentionIds,
              clientRequestId,
            },
          );
          if (sent.outcome === "choose")
            return { reason: sent.reason, candidates: sent.candidates };
          setState((s) => {
            const messages = withMessage(s.messages, sent.message);
            if (!sent.run) return { ...s, messages };
            const runs = applyStatus(s.runs, sent.run);
            return {
              ...s,
              messages,
              runs: sent.partial ? applySnapshot(runs, sent.partial) : runs,
            };
          });
          return "sent";
        } catch (error) {
          if (typeof error === "string") {
            setSendErrorKey(error);
            return "failed";
          }
        }
      }
      setSendErrorKey("unknown");
      return "failed";
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

  async function stopRoom() {
    setSendErrorKey(null);
    try {
      await invoke("stop_intervention", { roomId });
    } catch (error) {
      setSendErrorKey(String(error));
    }
  }

  async function changeRoster(command: string, args: Record<string, unknown>) {
    setRosterBusy(true);
    setRosterErrorKey(null);
    try {
      const roster = await invoke<Roster>(command, { roomId, ...args });
      setRoom((current) => current && { ...current, agents: roster.agents });
      setFormerAgents(roster.former_agents);
      return true;
    } catch (error) {
      setRosterErrorKey(String(error));
      return false;
    } finally {
      setRosterBusy(false);
    }
  }

  const addAgents = (agentIds: string[]) =>
    changeRoster("add_room_agents", { agentIds });
  const removeAgent = (agentId: string) =>
    changeRoster("remove_room_agent", { agentId });
  const clearRosterError = () => setRosterErrorKey(null);

  const proposal = room?.agents.some((agent) => agent.id === state.proposal)
    ? state.proposal
    : null;
  const detail = room
    ? {
        room,
        formerAgents,
        messages: state.messages,
        runs: state.runs,
        proposal,
      }
    : null;

  return {
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
  };
}
