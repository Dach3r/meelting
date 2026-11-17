import { invoke } from "@tauri-apps/api/core";
import { render, screen } from "@testing-library/react";
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

beforeEach(() => {
  invokeMock.mockReset();
});

describe("first launch", () => {
  it("creates the profile with the chosen language and workspace name", async () => {
    invokeMock.mockImplementation(async (command) =>
      command === "load_session" ? null : session(),
    );
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
    invokeMock.mockResolvedValue(session());
    render(<App />);

    expect(await screen.findByText("Tu oficina está lista")).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Navegación principal" })).toBeInTheDocument();
    expect(screen.getByText("Acme")).toBeInTheDocument();
  });

  it("switches to another workspace", async () => {
    invokeMock.mockImplementation(async (command) => {
      if (command === "load_session") return session({ locale: "en", workspaces: [acme, globex] });
      return session({ locale: "en", current_workspace_id: "2", workspaces: [acme, globex] });
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Switch workspace" }));
    await user.click(screen.getByRole("button", { name: /Globex/ }));

    expect(invokeMock).toHaveBeenCalledWith("switch_workspace", { workspaceId: "2" });
    expect(await screen.findByRole("button", { name: "Switch workspace" })).toHaveTextContent("Globex");
  });

  it("creates another workspace from the switcher", async () => {
    invokeMock.mockImplementation(async (command) => {
      if (command === "load_session") return session({ locale: "en" });
      return session({ locale: "en", current_workspace_id: "2", workspaces: [acme, globex] });
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Switch workspace" }));
    await user.click(screen.getByRole("button", { name: "Create another workspace" }));
    await user.type(screen.getByLabelText("Workspace name"), "Globex");
    await user.click(screen.getByRole("button", { name: "Create" }));

    expect(invokeMock).toHaveBeenCalledWith("create_workspace", { name: "Globex" });
    expect(await screen.findByRole("button", { name: "Switch workspace" })).toHaveTextContent("Globex");
  });

  it("closes the switcher when clicking outside or pressing Escape", async () => {
    invokeMock.mockResolvedValue(session({ locale: "en" }));
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
    invokeMock.mockResolvedValue(session({ locale: "en" }));
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
    invokeMock.mockResolvedValue(session({ locale: "en" }));
    render(<App />);

    expect(await screen.findByText("Teams")).toHaveAttribute("aria-current", "page");
    expect(screen.getByText("Rooms")).toHaveAttribute("aria-disabled", "true");
  });
});
