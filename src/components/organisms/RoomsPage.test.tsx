import { invoke } from "@tauri-apps/api/core";
import { listen, type EventCallback } from "@tauri-apps/api/event";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Message, Room } from "../../hooks/useRooms";
import type { Agent } from "../../hooks/useTeams";
import { I18nProvider } from "../../i18n/I18nProvider";
import { RoomsPage } from "./RoomsPage";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const invokeMock = vi.mocked(invoke);
const listenMock = vi.mocked(listen);

const ana: Agent = { id: "a1", team_id: "t1", name: "Ana", role: "QA Engineer", x: 0, y: 0 };
const bob: Agent = { id: "a2", team_id: "t2", name: "Bob", role: "Software Developer", x: 0, y: 0 };

const daily: Room = {
  id: "r1",
  name: "Daily",
  updated_at: "2026-11-18T10:00:00.000Z",
  agents: [{ id: "a1", name: "Ana", role: "QA Engineer" }],
};
const retro: Room = { id: "r2", name: "Retro", updated_at: "2026-11-17T10:00:00.000Z", agents: [] };

function message(id: string, body: string, overrides: Partial<Message> = {}): Message {
  return { id, room_id: "r1", agent_id: null, body, created_at: "2026-11-18T10:00:00.000Z", ...overrides };
}

let emit: EventCallback<Message> = () => {};

function core(handlers: Record<string, (args: Record<string, unknown>) => unknown>) {
  invokeMock.mockImplementation(async (command, args) => {
    const handler = handlers[command];
    if (!handler) throw new Error(`unexpected ${command}`);
    return handler((args ?? {}) as Record<string, unknown>);
  });
}

function renderPage(agents: Agent[] = [ana, bob]) {
  return render(
    <I18nProvider locale="en">
      <RoomsPage agents={agents} />
    </I18nProvider>,
  );
}

beforeEach(() => {
  invokeMock.mockReset();
  listenMock.mockReset();
  listenMock.mockImplementation(async (_event, handler) => {
    emit = handler as EventCallback<Message>;
    return () => {};
  });
});

describe("rooms list", () => {
  it("shows the empty state when there are no rooms", async () => {
    core({ list_rooms: () => ({ rooms: [], featured_messages: [] }) });
    renderPage();

    expect(await screen.findByText("No rooms yet")).toBeInTheDocument();
  });

  it("features the most recent room with its agents and latest messages and lists the rest", async () => {
    core({
      list_rooms: () => ({
        rooms: [daily, retro],
        featured_messages: [message("m1", "Hello team"), message("m2", "Ready", { agent_id: "a1" })],
      }),
    });
    renderPage();

    const featured = within(await screen.findByRole("region", { name: "Daily" }));
    expect(featured.getByText("1 agent")).toBeInTheDocument();
    expect(featured.getByText("Hello team")).toBeInTheDocument();
    expect(featured.getByText("You")).toBeInTheDocument();
    expect(featured.getAllByText("Ana")).toHaveLength(2);
    const others = within(screen.getByRole("list", { name: "Other rooms" }));
    expect(others.getByRole("button", { name: /Retro/ })).toHaveTextContent("No agents");
  });

  it("shows the translated error when the rooms fail to load", async () => {
    core({
      list_rooms: () => {
        throw "database";
      },
    });
    renderPage();

    expect(await screen.findByRole("alert")).toHaveTextContent("The local database failed. Try again.");
  });
});

describe("creating a room", () => {
  it("creates a room with the picked agents and opens it", async () => {
    core({
      list_rooms: () => ({ rooms: [], featured_messages: [] }),
      create_room: () => ({ ...daily, agents: [{ id: "a2", name: "Bob", role: "Software Developer" }] }),
      get_room: () => ({ room: { ...daily, agents: [{ id: "a2", name: "Bob", role: "Software Developer" }] }, messages: [] }),
    });
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "New room" }));
    const dialog = within(screen.getByRole("dialog", { name: "New room" }));
    await user.type(dialog.getByLabelText("Room name"), "Daily");
    await user.type(dialog.getByLabelText("Agents"), "an");
    await user.click(dialog.getByRole("option", { name: /Ana/ }));
    await user.click(dialog.getByRole("option", { name: /Bob/ }));
    expect(dialog.queryByRole("option")).not.toBeInTheDocument();
    await user.click(dialog.getByRole("button", { name: "Remove Ana" }));
    await user.click(dialog.getByRole("button", { name: "Create room" }));

    expect(invokeMock).toHaveBeenCalledWith("create_room", { name: "Daily", agentIds: ["a2"] });
    expect(await screen.findByRole("heading", { name: "Daily" })).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("get_room", { roomId: "r1" });
  });

  it("removes the last picked agent with Backspace on an empty search", async () => {
    core({ list_rooms: () => ({ rooms: [], featured_messages: [] }) });
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "New room" }));
    const dialog = within(screen.getByRole("dialog", { name: "New room" }));
    await user.type(dialog.getByLabelText("Agents"), "bo{Enter}");
    expect(dialog.getByRole("list", { name: "Selected agents" })).toHaveTextContent("Bob");
    await user.type(dialog.getByLabelText("Agents"), "{Backspace}");

    expect(dialog.queryByRole("list", { name: "Selected agents" })).not.toBeInTheDocument();
    expect(dialog.getByRole("option", { name: /Bob/ })).toBeInTheDocument();
  });

  it("says when the workspace has no agents and still allows a room without them", async () => {
    core({
      list_rooms: () => ({ rooms: [], featured_messages: [] }),
      create_room: () => retro,
      get_room: () => ({ room: retro, messages: [] }),
    });
    const user = userEvent.setup();
    renderPage([]);

    await user.click(await screen.findByRole("button", { name: "New room" }));
    expect(screen.getByText("There are no agents in this workspace yet.")).toBeInTheDocument();
    await user.type(screen.getByLabelText("Room name"), "Retro");
    await user.click(screen.getByRole("button", { name: "Create room" }));

    expect(invokeMock).toHaveBeenCalledWith("create_room", { name: "Retro", agentIds: [] });
    expect(await screen.findByText("No agents")).toBeInTheDocument();
  });

  it("keeps the form open with the translated error when the core rejects", async () => {
    core({
      list_rooms: () => ({ rooms: [], featured_messages: [] }),
      create_room: () => {
        throw "agent_not_found";
      },
    });
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "New room" }));
    await user.type(screen.getByLabelText("Room name"), "Daily");
    await user.click(screen.getByRole("button", { name: "Create room" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("That agent no longer exists.");
    expect(screen.getByRole("dialog", { name: "New room" })).toBeInTheDocument();
  });
});

describe("inside a room", () => {
  async function openDaily(handlers: Record<string, (args: Record<string, unknown>) => unknown> = {}) {
    core({
      list_rooms: () => ({ rooms: [daily], featured_messages: [] }),
      get_room: () => ({ room: daily, messages: [message("m1", "Hello team")] }),
      ...handlers,
    });
    const user = userEvent.setup();
    renderPage();
    await user.click(await screen.findByRole("button", { name: "Enter room" }));
    await screen.findByText("Hello team");
    return user;
  }

  it("shows you and the agents as participants and the conversation", async () => {
    await openDaily();

    const participants = within(screen.getByRole("region", { name: "Participants" }));
    expect(participants.getByText("You")).toBeInTheDocument();
    expect(participants.getByText("Ana")).toBeInTheDocument();
    expect(participants.getByText("QA Engineer")).toBeInTheDocument();
    expect(screen.queryByText(/Ask/)).not.toBeInTheDocument();
  });

  it("sends a message with Enter and shows it once even when its event also arrives", async () => {
    const sent = message("m2", "Let's start");
    const user = await openDaily({ send_message: () => sent });

    await user.type(screen.getByLabelText("Message"), "Let's start{Enter}");
    act(() => emit({ event: "message:created", id: 1, payload: sent }));

    expect(invokeMock).toHaveBeenCalledWith("send_message", { roomId: "r1", body: "Let's start" });
    expect(screen.getAllByText("Let's start")).toHaveLength(1);
    expect(screen.getByLabelText("Message")).toHaveValue("");
  });

  it("adds messages of this room that arrive as events and ignores other rooms", async () => {
    await openDaily();

    act(() => {
      emit({ event: "message:created", id: 1, payload: message("m3", "From the event") });
      emit({ event: "message:created", id: 2, payload: message("m4", "Elsewhere", { room_id: "r2" }) });
    });

    expect(screen.getByText("From the event")).toBeInTheDocument();
    expect(screen.queryByText("Elsewhere")).not.toBeInTheDocument();
  });

  it("keeps the text and shows the error when sending fails", async () => {
    const user = await openDaily({
      send_message: () => {
        throw "room_not_found";
      },
    });

    await user.type(screen.getByLabelText("Message"), "Anyone?");
    await user.click(screen.getByRole("button", { name: "Send" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("That room no longer exists.");
    expect(screen.getByLabelText("Message")).toHaveValue("Anyone?");
  });

  it("does not send a blank message", async () => {
    const user = await openDaily();

    await user.type(screen.getByLabelText("Message"), "   {Enter}");

    expect(screen.getByRole("button", { name: "Send" })).toBeDisabled();
    expect(invokeMock).not.toHaveBeenCalledWith("send_message", expect.anything());
  });

  it("deletes the room after confirming and goes back to the list", async () => {
    const user = await openDaily({ delete_room: () => ({ rooms: [], featured_messages: [] }) });

    await user.click(screen.getByRole("button", { name: "Delete room" }));
    const dialog = within(screen.getByRole("dialog", { name: "Delete room" }));
    expect(invokeMock).not.toHaveBeenCalledWith("delete_room", expect.anything());
    await user.click(dialog.getByRole("button", { name: "Delete room" }));

    expect(invokeMock).toHaveBeenCalledWith("delete_room", { roomId: "r1" });
    expect(await screen.findByText("No rooms yet")).toBeInTheDocument();
  });

  it("keeps the confirmation open with the error when deleting fails", async () => {
    const user = await openDaily({
      delete_room: () => {
        throw "room_not_found";
      },
    });

    await user.click(screen.getByRole("button", { name: "Delete room" }));
    const dialog = within(screen.getByRole("dialog", { name: "Delete room" }));
    await user.click(dialog.getByRole("button", { name: "Delete room" }));

    expect(await dialog.findByRole("alert")).toHaveTextContent("That room no longer exists.");
  });

  it("shows the error when the room no longer exists and goes back to a refreshed list", async () => {
    let rooms = [daily];
    core({
      list_rooms: () => ({ rooms, featured_messages: [] }),
      get_room: () => {
        throw "room_not_found";
      },
    });
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "Enter room" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("That room no longer exists.");
    rooms = [];
    await user.click(screen.getByRole("button", { name: "Rooms" }));

    await waitFor(() => expect(screen.getByText("No rooms yet")).toBeInTheDocument());
  });
});
