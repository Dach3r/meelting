import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

const acme = { id: "1", name: "Acme" };
const globex = { id: "2", name: "Globex" };
const roles = [
  { group: "product", roles: ["Product Owner"] },
  { group: "engineering", roles: ["Software Developer", "QA Engineer"] },
];

function session(overrides = {}) {
  return {
    locale: "es",
    theme: "system",
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
  delete document.documentElement.dataset.theme;
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

  it("selects a card on the board and clears the selection from the background", async () => {
    invokeMock.mockImplementation(async (command) =>
      command === "list_teams" ? [{ id: "t1", name: "General", agents: [] }] : session({ locale: "en" }),
    );
    const user = userEvent.setup();
    render(<App />);

    const card = await screen.findByRole("button", { name: "General" });
    expect(card).toHaveAttribute("aria-pressed", "false");
    await user.click(card);
    expect(card).toHaveAttribute("aria-pressed", "true");
    await user.click(screen.getByTestId("board"));
    expect(card).toHaveAttribute("aria-pressed", "false");
    await user.click(card);
    await user.click(card);
    expect(card).toHaveAttribute("aria-pressed", "false");
  });

  it("switches to the move tool, which grabs the board instead of selecting", async () => {
    invokeMock.mockImplementation(async (command) =>
      command === "list_teams" ? [{ id: "t1", name: "General", agents: [] }] : session({ locale: "en" }),
    );
    const user = userEvent.setup();
    render(<App />);

    const pan = await screen.findByRole("button", { name: "Move board" });
    const select = screen.getByRole("button", { name: "Select" });
    const board = screen.getByTestId("board");
    expect(select).toHaveAttribute("aria-pressed", "true");
    await user.click(pan);
    expect(pan).toHaveAttribute("aria-pressed", "true");
    expect(screen.queryByRole("button", { name: "General" })).not.toBeInTheDocument();
    expect(board).toHaveClass("cursor-grab");
    fireEvent.mouseDown(board, { clientX: 100, clientY: 100 });
    expect(board).toHaveClass("cursor-grabbing");
    fireEvent.mouseMove(board, { clientX: 60, clientY: 80 });
    fireEvent.mouseUp(board);
    expect(board).toHaveClass("cursor-grab");
    await user.click(select);
    expect(screen.getByRole("button", { name: "General" })).toBeInTheDocument();
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

  it("closes the team form from the close button of the modal", async () => {
    mockCore(session({ locale: "en" }));
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "New team" }));
    expect(screen.getByRole("dialog", { name: "New team" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Close" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
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
    expect(screen.queryByRole("button", { name: "Hire agent" })).not.toBeInTheDocument();
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
      if (command === "list_agent_templates") return [];
      if (command === "list_agent_roles") return roles;
      if (command === "create_agent") return withAgent;
      return session({ locale: "en" });
    });
    const user = userEvent.setup();
    render(<App />);

    const hireButtons = await screen.findAllByRole("button", { name: "Hire agent" });
    expect(hireButtons).toHaveLength(2);
    await user.click(hireButtons[0]);
    await user.click(await screen.findByRole("button", { name: "Create custom agent" }));
    expect(screen.queryByLabelText("Team")).not.toBeInTheDocument();
    const next = screen.getByRole("button", { name: "Continue" });
    expect(next).toBeDisabled();
    await user.type(screen.getByLabelText("Name"), "Ana");
    expect(within(screen.getByRole("group", { name: "Engineering" })).getAllByRole("option")).toHaveLength(2);
    await user.type(screen.getByLabelText("Search role"), "qa");
    expect(screen.getAllByRole("option")).toHaveLength(1);
    await user.clear(screen.getByLabelText("Search role"));
    await user.type(screen.getByLabelText("Search role"), "zzz");
    expect(screen.getByText("No role matches")).toBeInTheDocument();
    await user.clear(screen.getByLabelText("Search role"));
    await user.click(screen.getByRole("option", { name: "Software Developer" }));
    expect(screen.getByRole("option", { name: "Software Developer" })).toHaveAttribute("aria-selected", "true");
    await user.click(screen.getByRole("option", { name: "Software Developer" }));
    await user.click(next);
    expect(screen.getByRole("button", { name: "Continue" })).toBeDisabled();
    await user.type(screen.getByLabelText("Provider"), "openai");
    await user.type(screen.getByLabelText("Model"), "gpt-4o");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await user.click(screen.getByRole("button", { name: "Create agent" }));

    expect(invokeMock).toHaveBeenCalledWith("create_agent", {
      draft: {
        team_id: "t1",
        template_key: null,
        name: "Ana",
        role: "Software Developer",
        description: "",
        instructions: "",
        provider: "openai",
        model: "gpt-4o",
      },
    });
    expect(await screen.findByText("Ana")).toBeInTheDocument();
    expect(screen.queryByRole("listbox", { name: "Role" })).not.toBeInTheDocument();
  });
});

describe("agent catalog", () => {
  const templates = [
    {
      key: "product_owner",
      name: "Product Owner",
      role: "Product Owner",
      description: "Orders priorities.",
      instructions: "You are a Product Owner.\n\nHow you work:\n- Start with the problem.\n- Ask questions.",
    },
    {
      key: "qa_engineer",
      name: "QA Engineer",
      role: "QA Engineer",
      description: "Finds defects.",
      instructions: "You are a QA Engineer.",
    },
  ];
  const team = { id: "t1", name: "General", agents: [] };
  const hired = [{ ...team, agents: [{ id: "a1", team_id: "t1", name: "Ana", role: "Product Owner" }] }];

  function mockCatalog(createAgent: () => unknown = () => hired) {
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_teams") return [team];
      if (command === "list_agent_templates") return templates;
      if (command === "list_agent_roles") return roles;
      if (command === "create_agent") return createAgent();
      return session({ locale: "en" });
    });
  }

  async function openCatalog(user: ReturnType<typeof userEvent.setup>) {
    await user.click(await screen.findByRole("button", { name: "Hire agent" }));
  }

  it("returns to the teams from the Teams item of the rail while in the catalog", async () => {
    mockCatalog();
    const user = userEvent.setup();
    render(<App />);

    await openCatalog(user);
    expect(await screen.findByRole("heading", { name: "Agent catalog" })).toBeInTheDocument();
    const nav = within(screen.getByRole("navigation", { name: "Main navigation" }));
    await user.click(nav.getByRole("button", { name: "Teams" }));
    expect(screen.queryByRole("heading", { name: "Agent catalog" })).not.toBeInTheDocument();
  });

  it("lists the templates of the team catalog and goes back to the teams", async () => {
    mockCatalog();
    const user = userEvent.setup();
    render(<App />);

    await openCatalog(user);

    expect(await screen.findByRole("heading", { name: "Agent catalog" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Product Owner" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "QA Engineer" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Back to teams" }));
    expect(screen.queryByRole("heading", { name: "Agent catalog" })).not.toBeInTheDocument();
  });

  it("groups the templates in one row per area and hides empty areas", async () => {
    mockCatalog();
    const user = userEvent.setup();
    render(<App />);
    await openCatalog(user);

    const product = within(await screen.findByRole("region", { name: "Product" }));
    const engineering = within(screen.getByRole("region", { name: "Engineering" }));
    expect(product.getByRole("heading", { name: "Product Owner" })).toBeInTheDocument();
    expect(product.queryByRole("heading", { name: "QA Engineer" })).not.toBeInTheDocument();
    expect(engineering.getByRole("heading", { name: "QA Engineer" })).toBeInTheDocument();

    await user.type(screen.getByRole("searchbox", { name: "Search templates" }), "defects");
    expect(screen.queryByRole("region", { name: "Product" })).not.toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Engineering" })).toBeInTheDocument();
  });

  it("filters the templates by search text and by category", async () => {
    mockCatalog();
    const user = userEvent.setup();
    render(<App />);
    await openCatalog(user);
    await screen.findByRole("heading", { name: "Product Owner" });

    await user.type(screen.getByRole("searchbox", { name: "Search templates" }), "defects");
    expect(screen.queryByRole("heading", { name: "Product Owner" })).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "QA Engineer" })).toBeInTheDocument();

    await user.type(screen.getByRole("searchbox", { name: "Search templates" }), " nothing");
    expect(screen.getByText("No template matches")).toBeInTheDocument();

    await user.clear(screen.getByRole("searchbox", { name: "Search templates" }));
    await user.click(screen.getByRole("button", { name: "Product" }));
    expect(screen.getByRole("heading", { name: "Product Owner" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "QA Engineer" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "All" }));
    expect(screen.getByRole("heading", { name: "QA Engineer" })).toBeInTheDocument();
  });

  it("shows the template instructions by section", async () => {
    mockCatalog();
    const user = userEvent.setup();
    render(<App />);
    await openCatalog(user);

    await user.click(await screen.findByRole("button", { name: "View details Product Owner" }));

    const dialog = within(screen.getByRole("dialog", { name: "Product Owner" }));
    expect(dialog.getByRole("heading", { name: "How you work" })).toBeInTheDocument();
    expect(dialog.getByText("Start with the problem.")).toBeInTheDocument();
  });

  it("hires from a template through the wizard", async () => {
    mockCatalog();
    const user = userEvent.setup();
    render(<App />);
    await openCatalog(user);

    await user.click(await screen.findByRole("button", { name: "Hire Product Owner" }));
    expect(screen.getByLabelText("Name")).toHaveValue("Product Owner");
    await user.clear(screen.getByLabelText("Name"));
    await user.type(screen.getByLabelText("Name"), "Ana");
    expect(screen.getByRole("option", { name: "Product Owner" })).toHaveAttribute("aria-selected", "true");
    await user.click(screen.getByLabelText("Search role"));
    await user.keyboard("{ArrowDown}{Enter}");
    expect(screen.getByRole("option", { name: "Software Developer" })).toHaveAttribute("aria-selected", "true");
    await user.click(screen.getByRole("option", { name: "Product Owner" }));
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(screen.getByRole("button", { name: "Continue" })).toBeDisabled();
    await user.type(screen.getByLabelText("Provider"), "openai");
    await user.type(screen.getByLabelText("Model"), "gpt-4o");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(screen.getByLabelText("Description")).toHaveValue("Orders priorities.");
    expect(screen.getByLabelText("Instructions")).toHaveValue(templates[0].instructions);
    await user.click(screen.getByRole("button", { name: "Hire" }));

    expect(invokeMock).toHaveBeenCalledWith("create_agent", {
      draft: {
        team_id: "t1",
        template_key: "product_owner",
        name: "Ana",
        role: "Product Owner",
        description: "Orders priorities.",
        instructions: templates[0].instructions,
        provider: "openai",
        model: "gpt-4o",
      },
    });
    expect(await screen.findByText("Ana")).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Agent catalog" })).not.toBeInTheDocument();
  });

  it("keeps the wizard open with the error when the core rejects", async () => {
    mockCatalog(() => {
      throw "invalid_agent";
    });
    const user = userEvent.setup();
    render(<App />);
    await openCatalog(user);

    await user.click(await screen.findByRole("button", { name: "Hire QA Engineer" }));
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await user.type(screen.getByLabelText("Provider"), "openai");
    await user.type(screen.getByLabelText("Model"), "gpt-4o");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await user.click(screen.getByRole("button", { name: "Hire" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Check the fields");
    expect(screen.getByRole("button", { name: "Hire" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Agent catalog" })).toBeInTheDocument();
  });
});

describe("settings", () => {
  it("opens from the rail and marks Settings as the current page", async () => {
    mockCore(session({ locale: "en" }));
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Settings" }));

    expect(screen.getByRole("heading", { name: "Language" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Theme" })).not.toBeInTheDocument();
    const nav = within(screen.getByRole("navigation", { name: "Main navigation" }));
    expect(nav.getByText("Settings").closest("[aria-current]")).toHaveAttribute("aria-current", "page");
    await user.click(nav.getByRole("button", { name: "Teams" }));
    expect(screen.queryByRole("heading", { name: "Language" })).not.toBeInTheDocument();
  });

  it("switches between setting categories", async () => {
    mockCore(session({ locale: "en" }));
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Settings" }));
    const categories = within(screen.getByRole("navigation", { name: "Settings categories" }));
    expect(categories.getByRole("button", { name: "General" })).toHaveAttribute("aria-current", "true");

    await user.click(categories.getByRole("button", { name: "Appearance" }));

    expect(screen.getByRole("heading", { name: "Theme" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Language" })).not.toBeInTheDocument();
    expect(categories.getByRole("button", { name: "Appearance" })).toHaveAttribute("aria-current", "true");
  });

  it("changes the language and shows the app in it", async () => {
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_teams") return [];
      if (command === "change_locale") return session({ locale: "es" });
      return session({ locale: "en" });
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Settings" }));
    await user.click(screen.getByRole("button", { name: "ES" }));

    expect(invokeMock).toHaveBeenCalledWith("change_locale", { locale: "es" });
    expect(await screen.findByRole("heading", { name: "Idioma" })).toBeInTheDocument();
  });

  it("changes the theme and applies it to the document", async () => {
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_teams") return [];
      if (command === "change_theme") return session({ locale: "en", theme: "dark" });
      return session({ locale: "en" });
    });
    const user = userEvent.setup();
    render(<App />);

    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("system"));
    await user.click(await screen.findByRole("button", { name: "Settings" }));
    await user.click(screen.getByRole("button", { name: "Appearance" }));
    await user.click(screen.getByRole("button", { name: "Dark" }));

    expect(invokeMock).toHaveBeenCalledWith("change_theme", { theme: "dark" });
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("dark"));
    expect(screen.getByRole("button", { name: "Dark" })).toHaveAttribute("aria-current", "true");
  });

  it("shows the translated error when saving fails", async () => {
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_teams") return [];
      if (command === "change_theme") throw "database";
      return session({ locale: "en" });
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Settings" }));
    await user.click(screen.getByRole("button", { name: "Appearance" }));
    await user.click(screen.getByRole("button", { name: "Light" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("The local database failed. Try again.");
  });
});
