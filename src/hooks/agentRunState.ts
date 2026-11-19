export type RunStatus = "queued" | "running" | "succeeded" | "failed";

export type AgentRun = {
  id: string;
  room_id: string;
  agent_id: string;
  request_message_id: string;
  response_message_id: string | null;
  status: RunStatus;
  error_kind: string | null;
  attempts: number;
  provider: string;
  model: string;
  input_tokens: number | null;
  output_tokens: number | null;
  created_at: string;
  started_at: string | null;
  finished_at: string | null;
};

export type RunSnapshot = {
  run_id: string;
  attempt: number;
  seq: number;
  text: string;
};

export type TokenChunk = RunSnapshot & { room_id: string };

export type RunState = {
  runs: Record<string, AgentRun>;
  live: Record<string, RunSnapshot>;
};

export const emptyRunState: RunState = { runs: {}, live: {} };

export function isFinal(run: AgentRun) {
  return run.status === "succeeded" || run.status === "failed";
}

export function isActive(run: AgentRun) {
  return !isFinal(run);
}

export function lifecyclePosition(run: AgentRun) {
  if (isFinal(run)) return Number.POSITIVE_INFINITY;
  return run.status === "running" ? 2 * run.attempts - 1 : 2 * run.attempts;
}

export function applyStatus(state: RunState, run: AgentRun): RunState {
  const known = state.runs[run.id];
  if (
    known &&
    (isFinal(known) || lifecyclePosition(run) <= lifecyclePosition(known))
  )
    return state;
  const live = { ...state.live };
  if (isFinal(run)) delete live[run.id];
  return { runs: { ...state.runs, [run.id]: run }, live };
}

function isNewer(chunk: RunSnapshot, known: RunSnapshot | undefined) {
  if (!known) return true;
  return (
    chunk.attempt > known.attempt ||
    (chunk.attempt === known.attempt && chunk.seq > known.seq)
  );
}

export function applySnapshot(
  state: RunState,
  snapshot: RunSnapshot,
): RunState {
  const run = state.runs[snapshot.run_id];
  if ((run && isFinal(run)) || !isNewer(snapshot, state.live[snapshot.run_id]))
    return state;
  return { ...state, live: { ...state.live, [snapshot.run_id]: snapshot } };
}

export function applyToken(state: RunState, chunk: TokenChunk): RunState {
  const run = state.runs[chunk.run_id];
  const known = state.live[chunk.run_id];
  if (
    (run && (isFinal(run) || chunk.attempt < run.attempts)) ||
    !isNewer(chunk, known)
  )
    return state;
  const text =
    known && known.attempt === chunk.attempt
      ? known.text + chunk.text
      : chunk.text;
  return {
    ...state,
    live: {
      ...state.live,
      [chunk.run_id]: {
        run_id: chunk.run_id,
        attempt: chunk.attempt,
        seq: chunk.seq,
        text,
      },
    },
  };
}

export function liveText(state: RunState, run: AgentRun) {
  const live = state.live[run.id];
  return run.status === "running" &&
    live &&
    live.attempt === run.attempts &&
    live.text
    ? live.text
    : null;
}

export function loadRunState(
  runs: AgentRun[],
  snapshots: RunSnapshot[],
): RunState {
  const withRuns = runs.reduce(applyStatus, emptyRunState);
  return snapshots.reduce(applySnapshot, withRuns);
}
