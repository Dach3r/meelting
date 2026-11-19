import { describe, expect, it } from "vitest";
import {
  type AgentRun,
  type TokenChunk,
  applySnapshot,
  applyStatus,
  applyToken,
  emptyRunState,
  lifecyclePosition,
  liveText,
  loadRunState,
} from "./agentRunState";

function run(
  status: AgentRun["status"],
  attempts: number,
  overrides: Partial<AgentRun> = {},
): AgentRun {
  return {
    id: "run1",
    room_id: "r1",
    agent_id: "a1",
    request_message_id: "m1",
    response_message_id: null,
    status,
    error_kind: status === "failed" ? "cancelled" : null,
    attempts,
    provider: "ollama",
    model: "qwen",
    input_tokens: null,
    output_tokens: null,
    created_at: "2026-11-18T10:00:00.000Z",
    started_at: attempts > 0 ? "2026-11-18T10:00:01.000Z" : null,
    finished_at: null,
    ...overrides,
  };
}

function token(attempt: number, seq: number, text: string): TokenChunk {
  return { run_id: "run1", room_id: "r1", attempt, seq, text };
}

describe("lifecycle position", () => {
  it("orders every transition, including a retry within the same attempt", () => {
    const sequence = [
      run("queued", 0),
      run("running", 1),
      run("queued", 1),
      run("running", 2),
      run("queued", 2),
      run("running", 3),
    ];

    const positions = sequence.map(lifecyclePosition);

    expect(positions).toEqual([0, 1, 2, 3, 4, 5]);
    expect(lifecyclePosition(run("failed", 1))).toBeGreaterThan(5);
  });
});

describe("status events out of order", () => {
  it("a late running event does not undo the retry wait of the same attempt", () => {
    let state = applyStatus(emptyRunState, run("queued", 0));
    state = applyStatus(state, run("queued", 1));
    state = applyStatus(state, run("running", 1));

    expect(state.runs.run1.status).toBe("queued");
    expect(state.runs.run1.attempts).toBe(1);
  });

  it("an older attempt never replaces a newer one", () => {
    let state = applyStatus(emptyRunState, run("running", 2));
    state = applyStatus(state, run("queued", 1));
    state = applyStatus(state, run("running", 1));
    state = applyStatus(state, run("queued", 0));

    expect(state.runs.run1).toMatchObject({ status: "running", attempts: 2 });
  });

  it("a final state is never replaced, not even by another final state", () => {
    let state = applyStatus(
      emptyRunState,
      run("succeeded", 1, { response_message_id: "m2" }),
    );
    state = applyStatus(state, run("running", 1));
    state = applyStatus(state, run("failed", 1));

    expect(state.runs.run1.status).toBe("succeeded");
  });

  it("a final state drops the live text", () => {
    let state = applyStatus(emptyRunState, run("running", 1));
    state = applyToken(state, token(1, 1, "Hel"));
    state = applyStatus(state, run("failed", 1));

    expect(state.live.run1).toBeUndefined();
  });
});

describe("tokens", () => {
  it("appends tokens in sequence and ignores repeated or older ones", () => {
    let state = applyStatus(emptyRunState, run("running", 1));
    state = applyToken(state, token(1, 1, "Hel"));
    state = applyToken(state, token(1, 1, "Hel"));
    state = applyToken(state, token(1, 2, "lo"));
    state = applyToken(state, token(1, 1, "late"));

    expect(liveText(state, state.runs.run1)).toBe("Hello");
  });

  it("a newer attempt restarts the text", () => {
    let state = applyStatus(emptyRunState, run("running", 1));
    state = applyToken(state, token(1, 1, "first try"));
    state = applyStatus(state, run("queued", 1));
    expect(liveText(state, state.runs.run1)).toBeNull();
    state = applyStatus(state, run("running", 2));
    state = applyToken(state, token(2, 1, "second"));
    state = applyToken(state, token(1, 2, " stale"));

    expect(liveText(state, state.runs.run1)).toBe("second");
  });

  it("ignores tokens of finished runs and of attempts older than the run", () => {
    let state = applyStatus(emptyRunState, run("running", 2));
    state = applyToken(state, token(1, 5, "old attempt"));
    expect(state.live.run1).toBeUndefined();

    state = applyStatus(state, run("failed", 2));
    state = applyToken(state, token(2, 1, "after the end"));
    expect(state.live.run1).toBeUndefined();
  });

  it("keeps tokens that arrive before the run is known", () => {
    let state = applyToken(emptyRunState, token(1, 1, "early"));
    state = applyStatus(state, run("running", 1));

    expect(liveText(state, state.runs.run1)).toBe("early");
  });
});

describe("snapshot on load", () => {
  it("applies the snapshot and continues with the following tokens only", () => {
    let state = loadRunState(
      [run("running", 1)],
      [{ run_id: "run1", attempt: 1, seq: 3, text: "Hello" }],
    );
    state = applyToken(state, token(1, 3, "Hello"));
    state = applyToken(state, token(1, 4, " world"));

    expect(liveText(state, state.runs.run1)).toBe("Hello world");
  });

  it("an older snapshot does not replace newer tokens received during the load", () => {
    let state = applyStatus(emptyRunState, run("running", 1));
    state = applyToken(state, token(1, 4, "newer"));
    state = applySnapshot(state, {
      run_id: "run1",
      attempt: 1,
      seq: 2,
      text: "older",
    });

    expect(state.live.run1.seq).toBe(4);
  });

  it("ignores the snapshot of a run that is already finished in the database", () => {
    const state = loadRunState(
      [run("succeeded", 1, { response_message_id: "m2" })],
      [{ run_id: "run1", attempt: 1, seq: 1, text: "x" }],
    );

    expect(state.live.run1).toBeUndefined();
  });
});
