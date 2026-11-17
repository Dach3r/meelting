import { invoke } from "@tauri-apps/api/core";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

const acme = { id: "1", name: "Acme" };
const globex = { id: "2", name: "Globex" };

function session(overrides = {}) {
  return {
    locale: "es",
    current_workspace_id: "1",
    workspaces: [acme],
    ...overrides,
  };
}

function mockCore(value: unknown) {
  invokeMock.mockImplementation(async (command) => (command === "list_teams" ? [] : value));
}

beforeEach(() => {
  invokeMock.mockReset();
});

describe("first launch", () => {
  it("creates the profile with the chosen language and workspace name", async () => {
    invokeMock.mockImplementation(async (command) => {
      if (command === "load_session") return null;
      return command === "list_teams" ? [] : session();
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "ES" }));
    await user.type(screen.getByLabelText("Nombre del workspace"), "Acme");
    await user.click(screen.getByRole("button", { name: "Crear workspace" }));

    expect(invokeMock).toHaveBeenCalledWith("complete_first_launch", {
      locale: "es",
      workspaceName: "Acme",
    });
    expect(await screen.findByText("Tu oficina está lista")).toBeInTheDocument();
  });

  it("keeps the submit button disabled while the name is blank", async () => {
    invokeMock.mockResolvedValue(null);
    const user = userEvent.setup();
    render(<App />);

    const submit = await screen.findByRole("button", { name: "Create workspace" });
    expect(submit).toBeDisabled();
    await user.type(screen.getByLabelText("Workspace name"), "   ");
    expect(submit).toBeDisabled();
  });

  it("shows the translated error returned by the core", async () => {
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_teams") return [];
      if (command === "load_session") return null;
      throw "invalid_name";
    });
    const user = userEvent.setup();
    render(<App />);

    await user.type(await screen.findByLabelText("Workspace name"), "Acme");
    await user.click(screen.getByRole("button", { name: "Create workspace" }));

    expect(await screen.findAllByText("Enter a name of up to 80 characters.")).not.toHaveLength(0);
  });
});

describe("workspace shell", () => {
  it("opens the saved session in the saved language", async () => {
    mockCore(session());
    render(<App />);

    expect(await screen.findByText("Tu oficina está lista")).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Navegación principal" })).toBeInTheDocument();
    expect(screen.getAllByText("Acme")).not.toHaveLength(0);
  });

  it("switches to another workspace", async () => {
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_teams") return [];
      if (command === "load_session") return session({ locale: "en", workspaces: [acme, globex] });
      return session({ locale: "en", current_workspace_id: "2", workspaces: [acme, globex] });
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Switch workspace" }));
    await user.click(screen.getByRole("button", { name: /Globex/ }));

    expect(await screen.findByRole("status")).toHaveTextContent("Switching to Globex");
    expect(invokeMock).toHaveBeenCalledWith("switch_workspace", { workspaceId: "2" });
    expect(await screen.findByRole("button", { name: "Switch workspace" })).toHaveTextContent("Globex");
    await waitFor(() => expect(screen.queryByRole("status")).not.toBeInTheDocument());
  });

  it("shows the error in the overlay when switching fails and lets the user dismiss it", async () => {
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_teams") return [];
      if (command === "load_session") return session({ locale: "en", workspaces: [acme, globex] });
      throw "workspace_not_found";
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Switch workspace" }));
    await user.click(screen.getByRole("button", { name: /Globex/ }));

    expect(await screen.findByRole("alert")).toHaveTextContent("That workspace no longer exists.");
    await user.click(screen.getByRole("button", { name: "Close" }));

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Switch workspace" })).toHaveTextContent("Acme");
  });

  it("creates another workspace from its own page", async () => {
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_teams") return [];
      if (command === "load_session") return session({ locale: "en" });
      return session({ locale: "en", current_workspace_id: "2", workspaces: [acme, globex] });
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Switch workspace" }));
    await user.click(screen.getByRole("button", { name: "Create another workspace" }));
    await user.type(screen.getByLabelText("Workspace name"), "Globex");
    await user.click(screen.getByRole("button", { name: "Create workspace" }));

    expect(invokeMock).toHaveBeenCalledWith("create_workspace", { name: "Globex" });
    expect(await screen.findByRole("button", { name: "Switch workspace" })).toHaveTextContent("Globex");
  });

  it("goes back from the new workspace page without creating", async () => {
    mockCore(session({ locale: "en" }));
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Switch workspace" }));
    await user.click(screen.getByRole("button", { name: "Create another workspace" }));
    await user.click(screen.getByRole("button", { name: "Cancel" }));

    expect(await screen.findByRole("button", { name: "Switch workspace" })).toHaveTextContent("Acme");
    expect(invokeMock).not.toHaveBeenCalledWith("create_workspace", expect.anything());
  });

  it("closes the switcher when clicking outside or pressing Escape", async () => {
    mockCore(session({ locale: "en" }));
    const user = userEvent.setup();
    render(<App />);

    const trigger = await screen.findByRole("button", { name: "Switch workspace" });
    await user.click(trigger);
    expect(trigger).toHaveAttribute("aria-expanded", "true");

    await user.click(document.body);
    expect(trigger).toHaveAttribute("aria-expanded", "false");

    await user.click(trigger);
    await user.keyboard("{Escape}");
    expect(trigger).toHaveAttribute("aria-expanded", "false");
  });

  it("collapses the sidebar and remembers it", async () => {
    mockCore(session({ locale: "en" }));
    const user = userEvent.setup();
    const { container, unmount } = render(<App />);

    const toggle = await screen.findByRole("button", { name: "Collapse" });
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    await user.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(container.querySelector("[data-collapsed]")).not.toBeNull();

    unmount();
    render(<App />);
    expect(await screen.findByRole("button", { name: "Collapse" })).toHaveAttribute("aria-expanded", "false");
    localStorage.clear();
  });

  it("marks only Teams as the current page and the rest as coming soon", async () => {
    mockCore(session({ locale: "en" }));
    render(<App />);

    const nav = within(await screen.findByRole("navigation", { name: "Main navigation" }));
    expect(nav.getByText("Teams").closest("[aria-current]")).toHaveAttribute("aria-current", "page");
    expect(nav.getByText("Rooms").closest("[aria-disabled]")).toHaveAttribute("aria-disabled", "true");
  });

  it("lists the teams of the workspace with their agents", async () => {
    invokeMock.mockImplementation(async (command) =>
      command === "list_teams"
        ? [
            { id: "t1", name: "General", agents: [{ id: "a1", team_id: "t1", name: "Ana", role: "Developer" }] },
            { id: "t2", name: "Design", agents: [] },
          ]
        : session({ locale: "en" }),
    );
    render(<App />);

    expect(await screen.findByRole("heading", { name: "General" })).toBeInTheDocument();
    expect(screen.getByText("Ana")).toBeInTheDocument();
    expect(screen.getByText("Developer")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Design" })).toBeInTheDocument();
    expect(screen.getByText("No agents in this team yet.")).toBeInTheDocument();
  });

  it("draws the org chart with you at the root and the teams under it", async () => {
    invokeMock.mockImplementation(async (command) =>
      command === "list_teams"
        ? [{ id: "t1", name: "General", agents: [{ id: "a1", team_id: "t1", name: "Ana", role: "Developer" }] }]
        : session({ locale: "en" }),
    );
    render(<App />);

    expect(await screen.findByText("You")).toBeInTheDocument();
    expect(screen.getByText("Supervisor")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Show or hide You" })).toHaveTextContent("1");
    expect(screen.getByRole("button", { name: "Show or hide General" })).toHaveTextContent("1");
  });

  it("hides and shows the agents of a team", async () => {
    invokeMock.mockImplementation(async (command) =>
      command === "list_teams"
        ? [{ id: "t1", name: "General", agents: [{ id: "a1", team_id: "t1", name: "Ana", role: "Developer" }] }]
        : session({ locale: "en" }),
    );
    const user = userEvent.setup();
    render(<App />);

    const toggle = await screen.findByRole("button", { name: "Show or hide General" });
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    await user.click(toggle);
    expect(screen.queryByText("Ana")).not.toBeInTheDocument();
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    await user.click(toggle);
    expect(screen.getByText("Ana")).toBeInTheDocument();
  });

  it("hides and shows the teams from the root card", async () => {
    invokeMock.mockImplementation(async (command) =>
      command === "list_teams" ? [{ id: "t1", name: "General", agents: [] }] : session({ locale: "en" }),
    );
    const user = userEvent.setup();
    render(<App />);

    const toggle = await screen.findByRole("button", { name: "Show or hide You" });
    await user.click(toggle);
    expect(screen.queryByRole("heading", { name: "General" })).not.toBeInTheDocument();
    await user.click(toggle);
    expect(screen.getByRole("heading", { name: "General" })).toBeInTheDocument();
  });

  it("zooms the chart within its limits and resets it", async () => {
    invokeMock.mockImplementation(async (command) =>
      command === "list_teams" ? [{ id: "t1", name: "General", agents: [] }] : session({ locale: "en" }),
    );
    const user = userEvent.setup();
    render(<App />);

    const zoomIn = await screen.findByRole("button", { name: "Zoom in" });
    const reset = screen.getByRole("button", { name: "Reset zoom" });
    expect(reset).toHaveTextContent("100%");
    await user.click(zoomIn);
    expect(reset).toHaveTextContent("110%");
    for (let i = 0; i < 4; i++) await user.click(zoomIn);
    expect(reset).toHaveTextContent("140%");
    expect(zoomIn).toBeDisabled();
    await user.click(reset);
    expect(reset).toHaveTextContent("100%");
    const zoomOut = screen.getByRole("button", { name: "Zoom out" });
    for (let i = 0; i < 4; i++) await user.click(zoomOut);
    expect(reset).toHaveTextContent("60%");
    expect(zoomOut).toBeDisabled();
  });

  it("offers no zoom while there are no teams", async () => {
    mockCore(session({ locale: "en" }));
    render(<App />);

    expect(await screen.findByRole("button", { name: "New team" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Zoom in" })).not.toBeInTheDocument();
  });

  it("shows the empty state when the workspace has no teams", async () => {
    mockCore(session({ locale: "en" }));
    render(<App />);

    expect(await screen.findByText("Create your first team to start adding agents.")).toBeInTheDocument();
  });

  it("shows the translated error when the teams fail to load", async () => {
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_teams") throw "database";
      return session({ locale: "en" });
    });
    render(<App />);

    expect(await screen.findByRole("alert")).toHaveTextContent("The local database failed. Try again.");
  });

  it("creates a team from the form and shows it", async () => {
    const created = [{ id: "t1", name: "Design", agents: [] }];
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_teams") return [];
      if (command === "create_team") return created;
      return session({ locale: "en" });
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "New team" }));
    await user.type(screen.getByLabelText("Team name"), "Design");
    await user.click(screen.getByRole("button", { name: "Create team" }));

    expect(invokeMock).toHaveBeenCalledWith("create_team", { name: "Design" });
    expect(await screen.findByRole("heading", { name: "Design" })).toBeInTheDocument();
    expect(screen.queryByLabelText("Team name")).not.toBeInTheDocument();
  });

  it("keeps the team form open and shows the error when creating fails", async () => {
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_teams") return [];
      if (command === "create_team") throw "invalid_name";
      return session({ locale: "en" });
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "New team" }));
    await user.type(screen.getByLabelText("Team name"), "Design");
    await user.click(screen.getByRole("button", { name: "Create team" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Enter a name of up to 80 characters.");
    expect(screen.getByLabelText("Team name")).toBeInTheDocument();
  });

  it("offers no way to create an agent until a team exists", async () => {
    mockCore(session({ locale: "en" }));
    render(<App />);

    expect(await screen.findByRole("button", { name: "New team" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "New agent" })).not.toBeInTheDocument();
  });

  it("creates an agent from the selected team", async () => {
    const teams = [
      { id: "t1", name: "General", agents: [] },
      { id: "t2", name: "Design", agents: [] },
    ];
    const withAgent = [
      { id: "t1", name: "General", agents: [{ id: "a1", team_id: "t1", name: "Ana", role: "Developer" }] },
    ];
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_teams") return teams;
      if (command === "create_agent") return withAgent;
      return session({ locale: "en" });
    });
    const user = userEvent.setup();
    render(<App />);

    const newAgentButtons = await screen.findAllByRole("button", { name: "New agent" });
    expect(newAgentButtons).toHaveLength(2);
    await user.click(newAgentButtons[0]);
    expect(screen.queryByLabelText("Team")).not.toBeInTheDocument();
    const submit = screen.getByRole("button", { name: "Create agent" });
    expect(submit).toBeDisabled();
    await user.type(screen.getByLabelText("Name"), "Ana");
    await user.type(screen.getByLabelText("Role"), "Developer");
    await user.type(screen.getByLabelText("Provider"), "openai");
    await user.type(screen.getByLabelText("Model"), "gpt-4o");
    await user.click(submit);

    expect(invokeMock).toHaveBeenCalledWith("create_agent", {
      draft: {
        team_id: "t1",
        name: "Ana",
        role: "Developer",
        description: "",
        instructions: "",
        provider: "openai",
        model: "gpt-4o",
      },
    });
    expect(await screen.findByText("Ana")).toBeInTheDocument();
    expect(screen.queryByLabelText("Role")).not.toBeInTheDocument();
  });
});
