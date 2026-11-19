import { invoke } from "@tauri-apps/api/core";
import { listen, type EventCallback } from "@tauri-apps/api/event";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AgentRun, RunSnapshot } from "../../hooks/agentRunState";
import type { Message, Participant, Room } from "../../hooks/useRooms";
import type { Agent } from "../../hooks/useTeams";
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
  finished_at: null,
  agents: [
    {
      id: "a1",
      name: "Ana",
      role: "QA Engineer",
      provider: "ollama",
      model: "llama3",
    },
    {
      id: "a2",
      name: "Bob",
      role: "Developer",
      provider: "ollama",
      model: "llama3",
    },
  ],
};

const workspaceAgents: Agent[] = [
  { id: "a1", team_id: "t1", name: "Ana", role: "QA Engineer", x: 0, y: 0 },
  { id: "a2", team_id: "t1", name: "Bob", role: "Developer", x: 0, y: 0 },
  { id: "a3", team_id: "t1", name: "Cleo", role: "Designer", x: 0, y: 0 },
];

const providers = [
  { key: "ollama", name: "Ollama", location: "local", models: [] },
  {
    key: "groq",
    name: "Groq",
    location: "cloud",
    models: ["openai/gpt-oss-120b"],
  },
];

function message(
  id: string,
  body: string,
  overrides: Partial<Message> = {},
): Message {
  return {
    id,
    room_id: "r1",
    agent_id: null,
    body,
    created_at: "2026-11-18T10:00:00.000Z",
    ...overrides,
  };
}

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

function sent(
  request: Message,
  started: AgentRun | null,
  partial: RunSnapshot | null = null,
) {
  return { outcome: "sent", message: request, run: started, partial };
}

function emit(event: string, payload: unknown) {
  act(() =>
    handlers.get(event)?.({ event, id: 0, payload } as Parameters<
      EventCallback<unknown>
    >[0]),
  );
}

function token(seq: number, text: string, attempt = 1, roomId = "r1") {
  emit("agent_run:token", {
    run_id: "run1",
    room_id: roomId,
    attempt,
    seq,
    text,
  });
}

type Detail = {
  room: Room;
  former_agents: Participant[];
  messages: Message[];
  runs: AgentRun[];
  partials: RunSnapshot[];
  proposed_agent_id: string | null;
};

function core(
  handlers: Record<string, (args: Record<string, unknown>) => unknown>,
) {
  invokeMock.mockImplementation(async (command, args) => {
    if (command === "list_llm_providers") return providers;
    if (command === "list_tasks" && !handlers.list_tasks) return [];
    const handler = handlers[command];
    if (!handler) throw new Error(`unexpected ${command}`);
    return handler((args ?? {}) as Record<string, unknown>);
  });
}

function detail(overrides: Partial<Detail> = {}): Detail {
  return {
    room,
    former_agents: [],
    messages: [],
    runs: [],
    partials: [],
    proposed_agent_id: null,
    ...overrides,
  };
}

async function renderRoom() {
  render(
    <I18nProvider locale="en">
      <RoomView
        roomId="r1"
        agents={workspaceAgents}
        busy={false}
        finishErrorMessage={null}
        onBack={() => {}}
        onFinish={async () => {}}
        onClearFinishError={() => {}}
      />
    </I18nProvider>,
  );
  await screen.findByRole("heading", { name: "Daily" });
}

async function ask(
  user: ReturnType<typeof userEvent.setup>,
  agent: string,
  body: string,
) {
  await user.selectOptions(
    screen.getByRole("combobox", { name: "Ask" }),
    agent,
  );
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
  it("shows each participant's provider and model on its tile", async () => {
    core({ get_room: () => detail() });
    await renderRoom();

    const participants = screen.getByRole("region", { name: "Participants" });
    expect(within(participants).getAllByText("Ollama · llama3")).toHaveLength(
      2,
    );
  });

  it("sends an instruction to the chosen agent and shows that it is waiting", async () => {
    core({
      get_room: () => detail(),
      send_intervention: () =>
        sent(message("m1", "Review it"), run("queued", 0)),
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
    expect(screen.getByRole("status")).toHaveTextContent("Queued…");
    expect(screen.queryByText(/qwen2.5:0.5b/)).not.toBeInTheDocument();
    expect(
      within(screen.getByRole("region", { name: "Participants" })).getByText(
        "Queued",
      ),
    ).toBeInTheDocument();
  });

  it("sends from the message by default and lets you choose nobody explicitly", async () => {
    core({
      get_room: () => detail(),
      resolve_recipient: () => ({ kind: "nobody" }),
      send_intervention: () => sent(message("m1", "Hello"), null),
    });
    const user = userEvent.setup();
    await renderRoom();

    await user.type(screen.getByLabelText("Message"), "Hello{Enter}");
    await user.selectOptions(
      screen.getByRole("combobox", { name: "Ask" }),
      "No one, just a message",
    );
    await user.type(
      screen.getByLabelText("Message"),
      "@Ana just a note{Enter}",
    );

    const calls = invokeMock.mock.calls
      .filter(([command]) => command === "send_intervention")
      .map(([, args]) => args);
    expect(calls).toEqual([
      expect.objectContaining({
        body: "Hello",
        selection: { kind: "auto", agent_id: null },
        mentionIds: [],
      }),
      expect.objectContaining({
        body: "@Ana just a note",
        selection: { kind: "nobody" },
      }),
    ]);
  });

  it("retries a failed invocation with the same id and uses a new id for the next instruction", async () => {
    let calls = 0;
    core({
      get_room: () => detail(),
      send_intervention: () => {
        calls += 1;
        if (calls === 1) throw new Error("ipc closed");
        return sent(
          message(`m${calls}`, "Review it"),
          run("queued", 0, {
            id: `run${calls}`,
            request_message_id: `m${calls}`,
          }),
        );
      },
    });
    const user = userEvent.setup();
    await renderRoom();

    await ask(user, "Ana · QA Engineer", "Review it");
    await ask(user, "Bob · Developer", "Next one");

    const ids = invokeMock.mock.calls
      .filter(([command]) => command === "send_intervention")
      .map(([, args]) => (args as { clientRequestId: string }).clientRequestId);
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
        return sent(message("m1", "Review it"), run("running", 1), {
          run_id: "run1",
          attempt: 1,
          seq: 2,
          text: "Half",
        });
      },
    });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    await renderRoom();

    await user.selectOptions(
      screen.getByRole("combobox", { name: "Ask" }),
      "Ana · QA Engineer",
    );
    await user.type(screen.getByLabelText("Message"), "Review it{Enter}");
    await act(() => vi.advanceTimersByTimeAsync(15000));

    const ids = invokeMock.mock.calls
      .filter(([command]) => command === "send_intervention")
      .map(([, args]) => (args as { clientRequestId: string }).clientRequestId);
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
        if (calls === 1)
          return {
            outcome: "choose",
            reason: "conflict",
            candidates: ["a1", "a2"],
          };
        return sent(
          message("m1", "@Bob review it"),
          run("queued", 0, { agent_id: "a2" }),
        );
      },
    });
    const user = userEvent.setup();
    await renderRoom();

    await ask(user, "Ana · QA Engineer", "@Bob review it");
    const choice = within(
      screen.getByRole("group", {
        name: "The chosen recipient and the mention don't match. Who is it for?",
      }),
    );
    expect(
      screen.queryByText("@Bob review it", { selector: "li *" }),
    ).not.toBeInTheDocument();
    await user.click(choice.getByRole("button", { name: "Bob · Developer" }));

    expect(invokeMock).toHaveBeenLastCalledWith(
      "send_intervention",
      expect.objectContaining({
        body: "@Bob review it",
        selection: { kind: "confirmed", agent_id: "a2" },
      }),
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

    expect(screen.getByRole("alert")).toHaveTextContent(
      "That agent is still working on a request in this room.",
    );
    expect(screen.getByLabelText("Message")).toHaveValue("Review it");
  });

  it("does not let you ask an agent that is already working", async () => {
    core({
      get_room: () =>
        detail({
          messages: [message("m1", "First")],
          runs: [run("running", 1)],
        }),
    });
    const user = userEvent.setup();
    await renderRoom();

    await ask(user, "Ana · QA Engineer", "Second");

    expect(
      screen.getByText("Ana is still working on a request."),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Send" })).toBeDisabled();
  });
});

describe("mentioning an agent", () => {
  it("suggests only this room's agents by name or role and inserts the chosen one with the keyboard", async () => {
    core({
      get_room: () => detail(),
      resolve_recipient: () => ({ kind: "agent", agent_id: "a2" }),
      send_intervention: () =>
        sent(message("m1", "@Bob how?"), run("queued", 0, { agent_id: "a2" })),
    });
    const user = userEvent.setup();
    await renderRoom();
    const field = screen.getByRole("combobox", { name: "Message" });

    await user.type(field, "@");
    const list = screen.getByRole("listbox", { name: "Agents in the room" });
    expect(
      within(list)
        .getAllByRole("option")
        .map((option) => option.textContent),
    ).toEqual(["AAnaQA Engineer", "BBobDeveloper"]);
    await user.type(field, "dev");
    expect(within(list).getAllByRole("option")).toHaveLength(1);
    await user.keyboard("{Enter}");
    expect(field).toHaveValue("@Bob ");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    await user.type(field, "how?");
    expect(await screen.findByText("Bob will reply.")).toBeInTheDocument();
    await user.keyboard("{Enter}");

    expect(invokeMock).toHaveBeenCalledWith("resolve_recipient", {
      roomId: "r1",
      body: "@Bob how?",
      selection: { kind: "auto", agent_id: null },
      mentionIds: ["a2"],
    });
    expect(invokeMock).toHaveBeenCalledWith(
      "send_intervention",
      expect.objectContaining({
        body: "@Bob how?",
        selection: { kind: "auto", agent_id: null },
        mentionIds: ["a2"],
      }),
    );
  });

  it("moves through suggestions with the arrows and closes them with Escape", async () => {
    core({
      get_room: () => detail(),
      resolve_recipient: () => ({ kind: "nobody" }),
    });
    const user = userEvent.setup();
    await renderRoom();
    const field = screen.getByRole("combobox", { name: "Message" });

    await user.type(field, "Hi @");
    await user.keyboard("{ArrowDown}");
    expect(
      screen.getByRole("option", { name: /Bob/, selected: true }),
    ).toBeInTheDocument();
    await user.keyboard("{ArrowDown}");
    expect(
      screen.getByRole("option", { name: /Ana/, selected: true }),
    ).toBeInTheDocument();
    await user.keyboard("{Escape}");

    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(field).toHaveValue("Hi @");
  });

  it("forgets a chosen mention once its text is removed", async () => {
    core({
      get_room: () => detail(),
      resolve_recipient: () => ({ kind: "nobody" }),
      send_intervention: () => sent(message("m1", "x"), null),
    });
    const user = userEvent.setup();
    await renderRoom();
    const field = screen.getByRole("combobox", { name: "Message" });

    await user.type(field, "@a");
    await user.keyboard("{Enter}");
    await user.clear(field);
    await user.type(field, "Plain text{Enter}");

    expect(invokeMock).toHaveBeenLastCalledWith(
      "send_intervention",
      expect.objectContaining({ body: "Plain text", mentionIds: [] }),
    );
  });

  it("tells you who will reply, or why it must be settled, before sending", async () => {
    let resolution: unknown = { kind: "nobody" };
    core({ get_room: () => detail(), resolve_recipient: () => resolution });
    const user = userEvent.setup();
    await renderRoom();

    await user.type(screen.getByLabelText("Message"), "Note");
    expect(
      await screen.findByText(
        "Nobody will reply: it will be saved as a message. Mention an agent with @ to ask them.",
      ),
    ).toBeInTheDocument();
    resolution = {
      kind: "choose",
      reason: "ambiguous",
      candidates: ["a1", "a2"],
    };
    await user.type(screen.getByLabelText("Message"), "!");
    expect(
      await screen.findByText(
        "Several agents match that name: you can pick one when sending.",
      ),
    ).toBeInTheDocument();
  });

  it("does not send to a busy agent resolved from the message", async () => {
    core({
      get_room: () =>
        detail({
          messages: [message("m1", "First")],
          runs: [run("running", 1)],
        }),
      resolve_recipient: () => ({ kind: "agent", agent_id: "a1" }),
    });
    const user = userEvent.setup();
    await renderRoom();

    await user.type(screen.getByLabelText("Message"), "Ana, again");

    expect(
      await screen.findByText("Ana is still working on a request."),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Send" })).toBeDisabled();
  });
});

describe("following the conversation", () => {
  it("proposes the agent that replied last and sends the follow-up to it", async () => {
    core({
      get_room: () => detail({ proposed_agent_id: "a2" }),
      resolve_recipient: ({ selection }) => ({
        kind: "proposed",
        agent_id: (selection as { agent_id: string }).agent_id,
      }),
      send_intervention: () =>
        sent(message("m1", "Why PKCE?"), run("queued", 0, { agent_id: "a2" })),
    });
    const user = userEvent.setup();
    await renderRoom();

    expect(screen.getByText("Replying to Bob")).toBeInTheDocument();
    expect(screen.getByText("suggested")).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Ask" })).toHaveDisplayValue(
      "From the message",
    );
    await user.type(screen.getByLabelText("Message"), "Why PKCE?{Enter}");

    expect(invokeMock).toHaveBeenCalledWith(
      "send_intervention",
      expect.objectContaining({
        body: "Why PKCE?",
        selection: { kind: "auto", agent_id: "a2" },
      }),
    );
  });

  it("moves the proposal only when an agent's reply is saved", async () => {
    core({
      get_room: () =>
        detail({
          messages: [message("m1", "Hi")],
          runs: [run("running", 1)],
          proposed_agent_id: "a2",
        }),
    });
    await renderRoom();

    emit("agent_run:status", run("failed", 1, { error_kind: "timeout" }));
    expect(screen.getByText("Replying to Bob")).toBeInTheDocument();
    emit(
      "agent_run:status",
      run("succeeded", 1, { id: "run2", response_message_id: "m2" }),
    );

    expect(screen.getByText("Replying to Ana")).toBeInTheDocument();
  });

  it("lets you dismiss the proposal for one message or choose someone explicitly", async () => {
    core({
      get_room: () => detail({ proposed_agent_id: "a2" }),
      resolve_recipient: () => ({ kind: "nobody" }),
      send_intervention: () => sent(message("m1", "Note"), null),
    });
    const user = userEvent.setup();
    await renderRoom();

    await user.click(
      screen.getByRole("button", { name: "Don't send this message to Bob" }),
    );
    expect(screen.queryByText("Replying to Bob")).not.toBeInTheDocument();
    await user.type(screen.getByLabelText("Message"), "Note{Enter}");
    expect(invokeMock).toHaveBeenLastCalledWith(
      "send_intervention",
      expect.objectContaining({ selection: { kind: "auto", agent_id: null } }),
    );
    expect(screen.getByText("Replying to Bob")).toBeInTheDocument();

    await user.selectOptions(
      screen.getByRole("combobox", { name: "Ask" }),
      "Ana · QA Engineer",
    );
    expect(screen.queryByText("Replying to Bob")).not.toBeInTheDocument();
  });

  it("never proposes an agent that is not in the room", async () => {
    core({ get_room: () => detail({ proposed_agent_id: "gone" }) });
    await renderRoom();

    expect(screen.queryByText(/Replying to/)).not.toBeInTheDocument();
  });
});

describe("agent availability", () => {
  const checked = [
    {
      agent_id: "a1",
      provider: "ollama",
      availability: "unavailable",
      reason: "provider_unavailable",
    },
    {
      agent_id: "a2",
      provider: "groq",
      availability: "needs_setup",
      reason: "provider_not_configured",
    },
  ];

  function tile(name: string) {
    return within(
      within(screen.getByRole("region", { name: "Participants" }))
        .getByText(name)
        .closest("li") as HTMLElement,
    );
  }

  it("shows each agent's real availability with its reason", async () => {
    core({ get_room: () => detail(), check_agent_availability: () => checked });
    await renderRoom();

    expect(await tile("Ana").findByText("Unavailable")).toHaveAttribute(
      "title",
      "Ollama isn't running. Open it and try again.",
    );
    expect(tile("Bob").getByText("Needs setup")).toHaveAttribute(
      "title",
      "Groq has no API key in this workspace.",
    );
  });

  it("explains a lost connection and lets you check again", async () => {
    let answer: {
      agent_id: string;
      provider: string;
      availability: string;
      reason: string | null;
    }[] = [
      {
        agent_id: "a2",
        provider: "groq",
        availability: "needs_connection",
        reason: "provider_unavailable",
      },
    ];
    core({ get_room: () => detail(), check_agent_availability: () => answer });
    const user = userEvent.setup();
    await renderRoom();
    expect(await tile("Bob").findByText("Needs a connection")).toHaveAttribute(
      "title",
      "The last request to Groq could not connect. Check your internet connection.",
    );

    answer = [
      {
        agent_id: "a2",
        provider: "groq",
        availability: "available",
        reason: null,
      },
    ];
    await user.click(
      screen.getByRole("button", { name: "Check availability" }),
    );

    expect(await tile("Bob").findByText("Available")).toBeInTheDocument();
    expect(
      invokeMock.mock.calls.filter(
        ([command]) => command === "check_agent_availability",
      ),
    ).toHaveLength(2);
  });

  it("shows activity before availability and checks again when a run finishes", async () => {
    core({
      get_room: () =>
        detail({ messages: [message("m1", "Hi")], runs: [run("queued", 1)] }),
      check_agent_availability: () => checked,
    });
    await renderRoom();

    expect(await tile("Ana").findByText("Retrying…")).toBeInTheDocument();
    emit("agent_run:status", run("running", 2));
    expect(tile("Ana").getByText("Thinking…")).toBeInTheDocument();
    emit(
      "agent_run:status",
      run("failed", 2, { error_kind: "provider_unavailable" }),
    );

    expect(await tile("Ana").findByText("Unavailable")).toBeInTheDocument();
    expect(
      invokeMock.mock.calls.filter(
        ([command]) => command === "check_agent_availability",
      ),
    ).toHaveLength(2);
  });

  it("does not check availability while you type", async () => {
    core({
      get_room: () => detail(),
      check_agent_availability: () => checked,
      resolve_recipient: () => ({ kind: "nobody" }),
    });
    const user = userEvent.setup();
    await renderRoom();

    await user.type(screen.getByLabelText("Message"), "Some text");

    expect(
      invokeMock.mock.calls.filter(
        ([command]) => command === "check_agent_availability",
      ),
    ).toHaveLength(1);
  });
});

describe("reading the conversation", () => {
  it("shows the time of each message with the full date on hover", async () => {
    core({ get_room: () => detail({ messages: [message("m1", "Hello")] }) });
    await renderRoom();

    const time = screen.getByText(
      new Intl.DateTimeFormat("en", { timeStyle: "short" }).format(
        new Date("2026-11-18T10:00:00.000Z"),
      ),
    );
    expect(time).toHaveAttribute(
      "title",
      new Intl.DateTimeFormat("en", {
        dateStyle: "medium",
        timeStyle: "short",
      }).format(new Date("2026-11-18T10:00:00.000Z")),
    );
  });

  it("does not repeat a role that matches the agent's name", async () => {
    core({
      get_room: () =>
        detail({
          room: {
            ...room,
            agents: [
              {
                id: "a1",
                name: "Tech Lead",
                role: "tech lead",
                provider: "ollama",
                model: "llama3",
              },
              ...room.agents.slice(1),
            ],
          },
        }),
    });
    const user = userEvent.setup();
    await renderRoom();

    const tile = within(
      within(screen.getByRole("region", { name: "Participants" }))
        .getByText("Tech Lead")
        .closest("li") as HTMLElement,
    );
    expect(tile.queryByText("tech lead")).not.toBeInTheDocument();
    expect(
      screen.getByRole("option", { name: "Tech Lead" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("option", { name: "Bob · Developer" }),
    ).toBeInTheDocument();
    await user.type(screen.getByRole("combobox", { name: "Message" }), "@t");
    expect(
      within(screen.getByRole("listbox")).getByRole("option").textContent,
    ).toBe("TTech Lead");
  });
});

describe("following a run", () => {
  it("shows the text live and replaces it with the saved reply and its usage", async () => {
    core({
      get_room: () =>
        detail({
          messages: [message("m1", "Say hello")],
          runs: [run("queued", 0)],
        }),
    });
    await renderRoom();

    emit("agent_run:status", run("running", 1));
    expect(screen.getByRole("status")).toHaveTextContent("Thinking…");
    token(1, "Hel");
    token(2, "lo");
    expect(screen.getByRole("status")).toHaveTextContent("Writing…");
    expect(screen.getByText("Hello")).toBeInTheDocument();

    emit("message:created", message("m2", "Hello!", { agent_id: "a1" }));
    emit(
      "agent_run:status",
      run("succeeded", 1, {
        response_message_id: "m2",
        input_tokens: 103,
        output_tokens: 9,
        finished_at: "2026-11-18T10:00:03.000Z",
      }),
    );

    expect(screen.queryByText("Hello")).not.toBeInTheDocument();
    expect(screen.getByText("Hello!")).toBeInTheDocument();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(screen.getAllByText(/qwen2.5:0.5b/)).toHaveLength(1);
    expect(
      screen.getByText("Ollama · qwen2.5:0.5b · 103 in · 9 out tokens"),
    ).toBeInTheDocument();
    expect(screen.queryByText("Thinking…")).not.toBeInTheDocument();
  });

  it("a retry restarts the live text and a late event never goes back", async () => {
    core({
      get_room: () =>
        detail({
          messages: [message("m1", "Say hello")],
          runs: [run("running", 1)],
        }),
    });
    await renderRoom();
    token(1, "First try");

    emit("agent_run:status", run("queued", 1));
    emit("agent_run:status", run("running", 1));

    expect(screen.queryByText("First try")).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent(
      "Temporary problem. Retrying (attempt 2 of 3)…",
    );

    emit("agent_run:status", run("running", 2));
    token(1, "Second", 2);
    expect(screen.getByText("Second")).toBeInTheDocument();
  });

  it("recovers the text when reopening the room and applies events received during the load", async () => {
    let resolveRoom: (value: Detail) => void = () => {};
    core({
      get_room: () => new Promise<Detail>((resolve) => (resolveRoom = resolve)),
    });
    render(
      <I18nProvider locale="en">
        <RoomView
          roomId="r1"
          agents={workspaceAgents}
          busy={false}
          finishErrorMessage={null}
          onBack={() => {}}
          onFinish={async () => {}}
          onClearFinishError={() => {}}
        />
      </I18nProvider>,
    );
    await vi.waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("get_room", { roomId: "r1" }),
    );

    token(3, "Hello");
    token(4, " world");
    await act(async () =>
      resolveRoom(
        detail({
          messages: [message("m1", "Say hello")],
          runs: [run("running", 1)],
          partials: [{ run_id: "run1", attempt: 1, seq: 3, text: "Hello" }],
        }),
      ),
    );

    expect(await screen.findByText("Hello world")).toBeInTheDocument();
  });

  it("ignores events of other rooms", async () => {
    core({
      get_room: () =>
        detail({
          messages: [message("m1", "Say hello")],
          runs: [run("running", 1)],
        }),
    });
    await renderRoom();

    token(1, "Elsewhere", 1, "r2");
    emit(
      "agent_run:status",
      run("failed", 1, { room_id: "r2", error_kind: "timeout" }),
    );
    emit("message:created", message("x", "Other room", { room_id: "r2" }));

    expect(screen.queryByText("Elsewhere")).not.toBeInTheDocument();
    expect(screen.queryByText("Other room")).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Thinking…");
  });

  it("cancels the run", async () => {
    core({
      get_room: () =>
        detail({
          messages: [message("m1", "Say hello")],
          runs: [run("running", 1)],
        }),
      cancel_agent_run: () => null,
    });
    const user = userEvent.setup();
    await renderRoom();

    await user.click(screen.getByRole("button", { name: "Cancel" }));
    emit(
      "agent_run:status",
      run("failed", 1, {
        error_kind: "cancelled",
        finished_at: "2026-11-18T10:00:02.000Z",
      }),
    );

    expect(invokeMock).toHaveBeenCalledWith("cancel_agent_run", {
      runId: "run1",
    });
    expect(screen.getByRole("status")).toHaveTextContent("Cancelled.");
    expect(
      screen.queryByRole("button", { name: "Cancel" }),
    ).not.toBeInTheDocument();
  });

  it("explains failures with the provider name", async () => {
    core({
      get_room: () =>
        detail({
          messages: [
            message("m1", "One"),
            message("m2", "Two"),
            message("m3", "Three"),
          ],
          runs: [
            run("failed", 3, {
              error_kind: "provider_unavailable",
              finished_at: "x",
            }),
            run("failed", 1, {
              id: "run2",
              request_message_id: "m2",
              agent_id: "a2",
              provider: "groq",
              model: "openai/gpt-oss-120b",
              error_kind: "provider_not_configured",
              finished_at: "x",
            }),
            run("failed", 1, {
              id: "run3",
              request_message_id: "m3",
              error_kind: "context_too_large",
              finished_at: "x",
            }),
          ],
        }),
    });
    await renderRoom();

    const statuses = screen
      .getAllByRole("status")
      .map((status) => status.textContent);
    expect(statuses).toEqual([
      "Ollama isn't running. Open it and try again.",
      "Groq has no API key in this workspace.",
      "The agent's instructions and this request are too long for this model. Shorten the instructions or use another model.",
    ]);
  });
});

describe("people joining and leaving", () => {
  function participants() {
    return within(screen.getByRole("region", { name: "Participants" }));
  }

  it("invites agents who are not in the room yet", async () => {
    core({
      get_room: () => detail(),
      add_room_agents: () => ({
        agents: [
          ...room.agents,
          {
            id: "a3",
            name: "Cleo",
            role: "Designer",
            provider: "ollama",
            model: "llama3",
          },
        ],
        former_agents: [],
      }),
    });
    const user = userEvent.setup();
    await renderRoom();

    await user.click(
      participants().getByRole("button", { name: "Invite agents" }),
    );
    const dialog = within(
      screen.getByRole("dialog", { name: "Invite agents" }),
    );
    expect(
      dialog.queryByRole("option", { name: /Ana/ }),
    ).not.toBeInTheDocument();
    await user.click(dialog.getByRole("option", { name: /Cleo/ }));
    await user.click(dialog.getByRole("button", { name: "Invite" }));
    expect(invokeMock).not.toHaveBeenCalledWith(
      "add_room_agents",
      expect.anything(),
    );
    const confirm = within(
      screen.getByRole("dialog", { name: "Invite to the room?" }),
    );
    expect(
      confirm.getByText("These agents will join the room: Cleo."),
    ).toBeInTheDocument();
    await user.click(confirm.getByRole("button", { name: "Invite" }));

    expect(invokeMock).toHaveBeenCalledWith("add_room_agents", {
      roomId: "r1",
      agentIds: ["a3"],
    });
    expect(await participants().findByText("Cleo")).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("removes an agent and keeps its name on what it said", async () => {
    core({
      get_room: () =>
        detail({
          messages: [
            message("m1", "Hi"),
            message("m2", "Hello there", { agent_id: "a1" }),
          ],
        }),
      remove_room_agent: () => ({
        agents: [room.agents[1]],
        former_agents: [room.agents[0]],
      }),
    });
    const user = userEvent.setup();
    await renderRoom();

    await user.click(
      participants().getByRole("button", { name: "Remove Ana from the room" }),
    );
    expect(invokeMock).not.toHaveBeenCalledWith(
      "remove_room_agent",
      expect.anything(),
    );
    await user.click(
      within(screen.getByRole("dialog", { name: "Remove Ana?" })).getByRole(
        "button",
        { name: "Remove" },
      ),
    );

    expect(invokeMock).toHaveBeenCalledWith("remove_room_agent", {
      roomId: "r1",
      agentId: "a1",
    });
    await vi.waitFor(() =>
      expect(participants().queryByText("Ana")).not.toBeInTheDocument(),
    );
    expect(
      within(screen.getByRole("list", { name: "Messages" })).getByText("Ana"),
    ).toBeInTheDocument();
    expect(screen.getByText("1 agent")).toBeInTheDocument();
  });

  it("keeps the agent when the removal is cancelled", async () => {
    core({ get_room: () => detail() });
    const user = userEvent.setup();
    await renderRoom();

    await user.click(
      participants().getByRole("button", { name: "Remove Ana from the room" }),
    );
    await user.click(
      within(screen.getByRole("dialog", { name: "Remove Ana?" })).getByRole(
        "button",
        { name: "Cancel" },
      ),
    );

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith(
      "remove_room_agent",
      expect.anything(),
    );
    expect(participants().getByText("Ana")).toBeInTheDocument();
  });

  it("names agents who had already left when the room opens", async () => {
    core({
      get_room: () =>
        detail({
          room: { ...room, agents: [] },
          former_agents: [
            {
              id: "a1",
              name: "Ana",
              role: "QA Engineer",
              provider: "ollama",
              model: "llama3",
            },
          ],
          messages: [message("m1", "Done", { agent_id: "a1" })],
        }),
    });
    await renderRoom();

    expect(
      within(screen.getByRole("list", { name: "Messages" })).getByText("Ana"),
    ).toBeInTheDocument();
  });

  it("explains why an agent could not be removed", async () => {
    core({
      get_room: () => detail(),
      remove_room_agent: () => {
        throw "agent_not_found";
      },
    });
    const user = userEvent.setup();
    await renderRoom();

    await user.click(
      participants().getByRole("button", { name: "Remove Bob from the room" }),
    );
    await user.click(
      within(screen.getByRole("dialog", { name: "Remove Bob?" })).getByRole(
        "button",
        { name: "Remove" },
      ),
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "That agent no longer exists.",
    );
    expect(participants().getByText("Bob")).toBeInTheDocument();
  });
});

describe("creating tasks from the room", () => {
  const created = {
    id: "t1",
    title: "Fix login",
    description: "",
    status: "pending",
    priority: "medium",
    repository_id: null,
    base_ref: null,
    origin_room_id: "r1",
    origin_room_name: "Daily",
    origin_message_id: null,
    linked_rooms: [],
    agent_ids: [],
    worktree: null,
    completed_at: null,
    created_at: "2026-11-24T10:00:00.000Z",
    updated_at: "2026-11-24T10:00:00.000Z",
  };

  it("creates a task that keeps the room as its origin", async () => {
    const create = vi.fn(() => created);
    core({
      get_room: () => detail(),
      list_repositories: () => [],
      create_task: create,
    });
    const user = userEvent.setup();
    await renderRoom();

    await user.click(screen.getByRole("button", { name: "Create task" }));
    const dialog = within(screen.getByRole("dialog", { name: "New task" }));
    await user.type(
      await dialog.findByLabelText("Description"),
      "Fix login",
    );
    await user.click(dialog.getByRole("button", { name: "Create task" }));

    expect(create).toHaveBeenCalledWith({
      draft: expect.objectContaining({
        description: "Fix login",
        agent_ids: [],
      }),
      origin: { room_id: "r1", message_id: null },
    });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith(
      "send_intervention",
      expect.anything(),
    );
  });

  it("turns a message into a task with its text as the description", async () => {
    const create = vi.fn(() => ({ ...created, origin_message_id: "m1" }));
    core({
      get_room: () =>
        detail({
          messages: [
            message("m1", "We should fix the login timeout", {
              agent_id: "a1",
            }),
          ],
        }),
      list_repositories: () => [],
      create_task: create,
    });
    const user = userEvent.setup();
    await renderRoom();

    await user.click(screen.getByRole("button", { name: "Turn into task" }));
    const dialog = within(screen.getByRole("dialog", { name: "New task" }));
    expect(await dialog.findByLabelText("Description")).toHaveValue(
      "We should fix the login timeout",
    );
    await user.click(dialog.getByRole("button", { name: "Create task" }));

    expect(create).toHaveBeenCalledWith({
      draft: expect.objectContaining({
        description: "We should fix the login timeout",
      }),
      origin: { room_id: "r1", message_id: "m1" },
    });
  });

  it("links an existing task to the room instead of creating one", async () => {
    const link = vi.fn(() => ({
      ...created,
      origin_room_id: null,
      origin_room_name: null,
      linked_rooms: [{ room_id: "r1", room_name: "Daily", message_id: null }],
    }));
    const create = vi.fn();
    core({
      get_room: () => detail(),
      list_repositories: () => [],
      list_tasks: () => [
        { ...created, id: "t1", title: "Project status", origin_room_id: null },
        {
          ...created,
          id: "t2",
          title: "Already here",
          origin_room_id: "r1",
          origin_message_id: null,
        },
        { ...created, id: "t3", title: "Other work", origin_room_id: null },
      ],
      link_task_to_room: link,
      create_task: create,
    });
    const user = userEvent.setup();
    await renderRoom();

    await user.click(screen.getByRole("button", { name: "Create task" }));
    const dialog = within(screen.getByRole("dialog", { name: "New task" }));
    await user.click(dialog.getByRole("button", { name: "Link existing" }));
    expect(await dialog.findByText("Project status")).toBeInTheDocument();
    expect(dialog.queryByText("Already here")).not.toBeInTheDocument();
    await user.type(dialog.getByLabelText("Search tasks by title"), "other");
    expect(dialog.queryByText("Project status")).not.toBeInTheDocument();
    await user.click(dialog.getByRole("button", { name: /Other work/ }));

    expect(link).toHaveBeenCalledWith({
      taskId: "t3",
      origin: { room_id: "r1", message_id: null },
    });
    expect(create).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});

describe("room tasks tab", () => {
  const base = {
    description: "",
    status: "in_progress",
    priority: "medium",
    repository_id: null,
    base_ref: null,
    origin_message_id: null,
    agent_ids: [],
    worktree: null,
    completed_at: null,
    created_at: "2026-11-24T10:00:00.000Z",
    updated_at: "2026-11-24T10:00:00.000Z",
  };

  it("lists only the tasks tied to this room with their status", async () => {
    core({
      get_room: () => detail(),
      list_tasks: () => [
        {
          ...base,
          id: "t1",
          title: "Fix login",
          origin_room_id: "r1",
          origin_room_name: "Daily",
          linked_rooms: [],
        },
        {
          ...base,
          id: "t2",
          title: "Linked task",
          status: "blocked",
          origin_room_id: null,
          origin_room_name: null,
          linked_rooms: [
            { room_id: "r1", room_name: "Daily", message_id: null },
          ],
        },
        {
          ...base,
          id: "t3",
          title: "Elsewhere",
          origin_room_id: "r9",
          origin_room_name: "Other",
          linked_rooms: [],
        },
      ],
    });
    const user = userEvent.setup();
    await renderRoom();

    await user.click(screen.getByRole("button", { name: "Tasks" }));

    expect(await screen.findByText("Fix login")).toBeInTheDocument();
    expect(screen.getByText("Linked task")).toBeInTheDocument();
    expect(screen.getByText("Blocked")).toBeInTheDocument();
    expect(screen.queryByText("Elsewhere")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Message")).not.toBeInTheDocument();
  });
});
