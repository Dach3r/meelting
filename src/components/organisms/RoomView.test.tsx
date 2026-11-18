import { invoke } from "@tauri-apps/api/core";
import { listen, type EventCallback } from "@tauri-apps/api/event";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AgentRun, RunSnapshot } from "../../hooks/agentRunState";
import type { Message, Room } from "../../hooks/useRooms";
import { I18nProvider } from "../../i18n/I18nProvider";
import { RoomView } from "./RoomView";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const invokeMock = vi.mocked(invoke);
const listenMock = vi.mocked(listen);
const handlers = new Map<string, EventCallback<unknown>>();
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

const room: Room = {
  id: "r1",
  name: "Daily",
  updated_at: "2026-11-18T10:00:00.000Z",
  agents: [
    { id: "a1", name: "Ana", role: "QA Engineer" },
    { id: "a2", name: "Bob", role: "Developer" },
  ],
};

const providers = [
  { key: "ollama", name: "Ollama", location: "local", models: [] },
  { key: "groq", name: "Groq", location: "cloud", models: ["openai/gpt-oss-120b"] },
];

function message(id: string, body: string, overrides: Partial<Message> = {}): Message {
  return { id, room_id: "r1", agent_id: null, body, created_at: "2026-11-18T10:00:00.000Z", ...overrides };
}

function run(status: AgentRun["status"], attempts: number, overrides: Partial<AgentRun> = {}): AgentRun {
  return {
    id: "run1",
    room_id: "r1",
    agent_id: "a1",
    request_message_id: "m1",
    response_message_id: null,
    status,
    error_kind: null,
    attempts,
    provider: "ollama",
    model: "qwen2.5:0.5b",
    input_tokens: null,
    output_tokens: null,
    created_at: "2026-11-18T10:00:00.000Z",
    started_at: attempts > 0 ? "2026-11-18T10:00:01.000Z" : null,
    finished_at: null,
    ...overrides,
  };
}

function sent(request: Message, started: AgentRun | null, partial: RunSnapshot | null = null) {
  return { outcome: "sent", message: request, run: started, partial };
}

function emit(event: string, payload: unknown) {
  act(() => handlers.get(event)?.({ event, id: 0, payload } as Parameters<EventCallback<unknown>>[0]));
}

function token(seq: number, text: string, attempt = 1, roomId = "r1") {
  emit("agent_run:token", { run_id: "run1", room_id: roomId, attempt, seq, text });
}

type Detail = { room: Room; messages: Message[]; runs: AgentRun[]; partials: RunSnapshot[] };

function core(handlers: Record<string, (args: Record<string, unknown>) => unknown>) {
  invokeMock.mockImplementation(async (command, args) => {
    if (command === "list_llm_providers") return providers;
    const handler = handlers[command];
    if (!handler) throw new Error(`unexpected ${command}`);
    return handler((args ?? {}) as Record<string, unknown>);
  });
}

function detail(overrides: Partial<Detail> = {}): Detail {
  return { room, messages: [], runs: [], partials: [], ...overrides };
}

async function renderRoom() {
  render(
    <I18nProvider locale="en">
      <RoomView roomId="r1" busy={false} deleteErrorMessage={null} onBack={() => {}} onDelete={async () => {}} onClearDeleteError={() => {}} />
    </I18nProvider>,
  );
  await screen.findByRole("heading", { name: "Daily" });
}

async function ask(user: ReturnType<typeof userEvent.setup>, agent: string, body: string) {
  await user.selectOptions(screen.getByRole("combobox", { name: "Ask" }), agent);
  await user.type(screen.getByLabelText("Message"), `${body}{Enter}`);
}

beforeEach(() => {
  invokeMock.mockReset();
  listenMock.mockReset();
  handlers.clear();
  listenMock.mockImplementation(async (event, handler) => {
    handlers.set(event as string, handler as EventCallback<unknown>);
    return () => {};
  });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("asking an agent", () => {
  it("sends an instruction to the chosen agent and shows that it is waiting", async () => {
    core({
      get_room: () => detail(),
      send_intervention: () => sent(message("m1", "Review it"), run("queued", 0)),
    });
    const user = userEvent.setup();
    await renderRoom();

    await ask(user, "Ana · QA Engineer", "Review it");

    expect(invokeMock).toHaveBeenCalledWith("send_intervention", {
      roomId: "r1",
      body: "Review it",
      selection: { kind: "agent", agent_id: "a1" },
      mentionIds: [],
      clientRequestId: expect.stringMatching(uuid),
    });
    expect(screen.getByText("Review it")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Waiting for Ana…");
    expect(screen.getByText("Ollama · qwen2.5:0.5b")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "Participants" })).getByText("Thinking…")).toBeInTheDocument();
  });

  it("an empty «Ask» still sends a plain message", async () => {
    core({ get_room: () => detail(), send_intervention: () => sent(message("m1", "Hello"), null) });
    const user = userEvent.setup();
    await renderRoom();

    await user.type(screen.getByLabelText("Message"), "Hello{Enter}");

    expect(invokeMock).toHaveBeenCalledWith("send_intervention", expect.objectContaining({ body: "Hello", selection: { kind: "nobody" } }));
  });

  it("retries a failed invocation with the same id and uses a new id for the next instruction", async () => {
    let calls = 0;
    core({
      get_room: () => detail(),
      send_intervention: () => {
        calls += 1;
        if (calls === 1) throw new Error("ipc closed");
        return sent(message(`m${calls}`, "Review it"), run("queued", 0, { id: `run${calls}`, request_message_id: `m${calls}` }));
      },
    });
    const user = userEvent.setup();
    await renderRoom();

    await ask(user, "Ana · QA Engineer", "Review it");
    await ask(user, "Bob · Developer", "Next one");

    const ids = invokeMock.mock.calls.filter(([command]) => command === "send_intervention").map(([, args]) => (args as { clientRequestId: string }).clientRequestId);
    expect(ids).toHaveLength(3);
    expect(ids[1]).toBe(ids[0]);
    expect(ids[2]).not.toBe(ids[0]);
  });

  it("recovers the original run when the invocation times out", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    let calls = 0;
    core({
      get_room: () => detail(),
      send_intervention: () => {
        calls += 1;
        if (calls === 1) return new Promise(() => {});
        return sent(message("m1", "Review it"), run("running", 1), { run_id: "run1", attempt: 1, seq: 2, text: "Half" });
      },
    });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    await renderRoom();

    await user.selectOptions(screen.getByRole("combobox", { name: "Ask" }), "Ana · QA Engineer");
    await user.type(screen.getByLabelText("Message"), "Review it{Enter}");
    await act(() => vi.advanceTimersByTimeAsync(15000));

    const ids = invokeMock.mock.calls.filter(([command]) => command === "send_intervention").map(([, args]) => (args as { clientRequestId: string }).clientRequestId);
    expect(ids).toEqual([ids[0], ids[0]]);
    expect(await screen.findByText("Half")).toBeInTheDocument();
    expect(screen.getAllByText("Review it")).toHaveLength(1);
  });

  it("asks who the message is for when the core finds a conflict and sends the confirmed choice", async () => {
    let calls = 0;
    core({
      get_room: () => detail(),
      send_intervention: () => {
        calls += 1;
        if (calls === 1) return { outcome: "choose", reason: "conflict", candidates: ["a1", "a2"] };
        return sent(message("m1", "@Bob review it"), run("queued", 0, { agent_id: "a2" }));
      },
    });
    const user = userEvent.setup();
    await renderRoom();

    await ask(user, "Ana · QA Engineer", "@Bob review it");
    const choice = within(screen.getByRole("group", { name: "The chosen recipient and the mention don't match. Who is it for?" }));
    expect(screen.queryByText("@Bob review it", { selector: "li *" })).not.toBeInTheDocument();
    await user.click(choice.getByRole("button", { name: "Bob · Developer" }));

    expect(invokeMock).toHaveBeenLastCalledWith(
      "send_intervention",
      expect.objectContaining({ body: "@Bob review it", selection: { kind: "confirmed", agent_id: "a2" } }),
    );
    expect(screen.queryByRole("group")).not.toBeInTheDocument();
    expect(screen.getByLabelText("Message")).toHaveValue("");
  });

  it("shows command errors translated and keeps the text", async () => {
    core({
      get_room: () => detail(),
      send_intervention: () => {
        throw "agent_busy";
      },
    });
    const user = userEvent.setup();
    await renderRoom();

    await ask(user, "Ana · QA Engineer", "Review it");

    expect(screen.getByRole("alert")).toHaveTextContent("That agent is still working on a request in this room.");
    expect(screen.getByLabelText("Message")).toHaveValue("Review it");
  });

  it("does not let you ask an agent that is already working", async () => {
    core({ get_room: () => detail({ messages: [message("m1", "First")], runs: [run("running", 1)] }) });
    const user = userEvent.setup();
    await renderRoom();

    await ask(user, "Ana · QA Engineer", "Second");

    expect(screen.getByText("Ana is still working on a request.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Send" })).toBeDisabled();
  });
});

describe("following a run", () => {
  it("shows the text live and replaces it with the saved reply and its usage", async () => {
    core({ get_room: () => detail({ messages: [message("m1", "Say hello")], runs: [run("queued", 0)] }) });
    await renderRoom();

    emit("agent_run:status", run("running", 1));
    token(1, "Hel");
    token(2, "lo");
    expect(screen.getByRole("status")).toHaveTextContent("Ana is working…");
    expect(screen.getByText("Hello")).toBeInTheDocument();

    emit("message:created", message("m2", "Hello!", { agent_id: "a1" }));
    emit("agent_run:status", run("succeeded", 1, { response_message_id: "m2", input_tokens: 103, output_tokens: 9, finished_at: "2026-11-18T10:00:03.000Z" }));

    expect(screen.queryByText("Hello")).not.toBeInTheDocument();
    expect(screen.getByText("Hello!")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Ana replied");
    expect(screen.getByText("Ollama · qwen2.5:0.5b · 103 in · 9 out tokens")).toBeInTheDocument();
    expect(screen.queryByText("Thinking…")).not.toBeInTheDocument();
  });

  it("a retry restarts the live text and a late event never goes back", async () => {
    core({ get_room: () => detail({ messages: [message("m1", "Say hello")], runs: [run("running", 1)] }) });
    await renderRoom();
    token(1, "First try");

    emit("agent_run:status", run("queued", 1));
    emit("agent_run:status", run("running", 1));

    expect(screen.queryByText("First try")).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Ana hit a temporary problem. Retrying (attempt 2 of 3)…");

    emit("agent_run:status", run("running", 2));
    token(1, "Second", 2);
    expect(screen.getByText("Second")).toBeInTheDocument();
  });

  it("recovers the text when reopening the room and applies events received during the load", async () => {
    let resolveRoom: (value: Detail) => void = () => {};
    core({ get_room: () => new Promise<Detail>((resolve) => (resolveRoom = resolve)) });
    render(
      <I18nProvider locale="en">
        <RoomView roomId="r1" busy={false} deleteErrorMessage={null} onBack={() => {}} onDelete={async () => {}} onClearDeleteError={() => {}} />
      </I18nProvider>,
    );
    await vi.waitFor(() => expect(invokeMock).toHaveBeenCalledWith("get_room", { roomId: "r1" }));

    token(3, "Hello");
    token(4, " world");
    await act(async () => resolveRoom(detail({ messages: [message("m1", "Say hello")], runs: [run("running", 1)], partials: [{ run_id: "run1", attempt: 1, seq: 3, text: "Hello" }] })));

    expect(await screen.findByText("Hello world")).toBeInTheDocument();
  });

  it("ignores events of other rooms", async () => {
    core({ get_room: () => detail({ messages: [message("m1", "Say hello")], runs: [run("running", 1)] }) });
    await renderRoom();

    token(1, "Elsewhere", 1, "r2");
    emit("agent_run:status", run("failed", 1, { room_id: "r2", error_kind: "timeout" }));
    emit("message:created", message("x", "Other room", { room_id: "r2" }));

    expect(screen.queryByText("Elsewhere")).not.toBeInTheDocument();
    expect(screen.queryByText("Other room")).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Ana is working…");
  });

  it("cancels the run", async () => {
    core({
      get_room: () => detail({ messages: [message("m1", "Say hello")], runs: [run("running", 1)] }),
      cancel_agent_run: () => null,
    });
    const user = userEvent.setup();
    await renderRoom();

    await user.click(screen.getByRole("button", { name: "Cancel" }));
    emit("agent_run:status", run("failed", 1, { error_kind: "cancelled", finished_at: "2026-11-18T10:00:02.000Z" }));

    expect(invokeMock).toHaveBeenCalledWith("cancel_agent_run", { runId: "run1" });
    expect(screen.getByRole("status")).toHaveTextContent("Cancelled.");
    expect(screen.queryByRole("button", { name: "Cancel" })).not.toBeInTheDocument();
  });

  it("explains failures with the provider name", async () => {
    core({
      get_room: () =>
        detail({
          messages: [message("m1", "One"), message("m2", "Two"), message("m3", "Three")],
          runs: [
            run("failed", 3, { error_kind: "provider_unavailable", finished_at: "x" }),
            run("failed", 1, { id: "run2", request_message_id: "m2", agent_id: "a2", provider: "groq", model: "openai/gpt-oss-120b", error_kind: "provider_not_configured", finished_at: "x" }),
            run("failed", 1, { id: "run3", request_message_id: "m3", error_kind: "context_too_large", finished_at: "x" }),
          ],
        }),
    });
    await renderRoom();

    const statuses = screen.getAllByRole("status").map((status) => status.textContent);
    expect(statuses).toEqual([
      "Ollama isn't running. Open it and try again.",
      "Groq has no API key in this workspace.",
      "The agent's instructions and this request are too long for this model. Shorten the instructions or use another model.",
    ]);
  });
});
