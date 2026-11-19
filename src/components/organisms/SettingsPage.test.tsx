import { invoke } from "@tauri-apps/api/core";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { I18nProvider } from "../../i18n/I18nProvider";
import { SettingsPage } from "./SettingsPage";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

function core(handlers: Record<string, (args?: unknown) => unknown>) {
  invokeMock.mockImplementation(async (command, args) => {
    const handler = handlers[command];
    if (!handler) throw new Error(`unexpected ${command}`);
    return handler(args);
  });
}

function callbacks() {
  return {
    onChangeLocale: vi.fn(),
    onChangeTheme: vi.fn(),
    onChangeUiScale: vi.fn(),
    onChangeLoadingEnabled: vi.fn(),
    onChangeTimeFormat: vi.fn(),
    onChangeReduceMotion: vi.fn(),
    onChangeNotification: vi.fn(),
  };
}

function renderPage(handlers = callbacks()) {
  render(
    <I18nProvider locale="en">
      <SettingsPage
        locale="en"
        theme="system"
        uiScale={100}
        loadingEnabled
        timeFormat="auto"
        reduceMotion="system"
        notifyRunSucceeded
        notifyRunFailed={false}
        notifySound={false}
        busy={false}
        errorMessage={null}
        {...handlers}
      />
    </I18nProvider>,
  );
  return handlers;
}

const nav = () =>
  within(screen.getByRole("navigation", { name: "Settings categories" }));

beforeEach(() => {
  invokeMock.mockReset();
  core({ get_open_at_login: () => false, get_worktrees_root: () => null });
});

describe("settings navigation", () => {
  it("lists the sections without a title or group headings and shows how many settings each has", () => {
    renderPage();
    expect(nav().queryByRole("heading")).not.toBeInTheDocument();
    expect(nav().getByRole("button", { name: "General" })).toHaveTextContent(
      "3",
    );
    expect(nav().getByRole("button", { name: "Appearance" })).toHaveTextContent(
      "4",
    );
    expect(
      nav().getByRole("button", { name: "Notifications" }),
    ).toHaveTextContent("3");
    expect(
      nav().getByRole("button", { name: "Environments" }),
    ).toHaveTextContent("1");
    expect(nav().getByRole("button", { name: "Advanced" })).toHaveTextContent(
      "2",
    );
  });

  it("marks the open section and switches between them", async () => {
    renderPage();
    const user = userEvent.setup();
    expect(nav().getByRole("button", { name: "General" })).toHaveAttribute(
      "aria-current",
      "true",
    );
    expect(
      screen.getByRole("heading", { level: 2, name: "General" }),
    ).toBeInTheDocument();

    await user.click(nav().getByRole("button", { name: "Notifications" }));

    expect(nav().getByRole("button", { name: "General" })).not.toHaveAttribute(
      "aria-current",
    );
    expect(
      screen.getByRole("switch", { name: "Run finished" }),
    ).toBeInTheDocument();
  });
});

describe("general", () => {
  it("shows the groups and changes the time format", async () => {
    const handlers = renderPage();
    const user = userEvent.setup();
    expect(screen.getByText("Language and region")).toBeInTheDocument();
    expect(screen.getByText("Startup")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "24 h" }));

    expect(handlers.onChangeTimeFormat).toHaveBeenCalledWith("24h");
  });

  it("reads and changes opening at login through the core", async () => {
    core({
      get_open_at_login: () => false,
      change_open_at_login: (args) => (args as { enabled: boolean }).enabled,
    });
    renderPage();
    const user = userEvent.setup();
    const toggle = screen.getByRole("switch", { name: "Open at login" });
    await waitFor(() => expect(toggle).toBeEnabled());
    expect(toggle).toHaveAttribute("aria-checked", "false");

    await user.click(toggle);

    expect(invokeMock).toHaveBeenCalledWith("change_open_at_login", {
      enabled: true,
    });
    await waitFor(() => expect(toggle).toHaveAttribute("aria-checked", "true"));
  });

  it("shows a translated error when opening at login cannot change", async () => {
    core({
      get_open_at_login: () => false,
      change_open_at_login: () => {
        throw "database";
      },
    });
    renderPage();
    const user = userEvent.setup();
    const toggle = screen.getByRole("switch", { name: "Open at login" });
    await waitFor(() => expect(toggle).toBeEnabled());

    await user.click(toggle);

    expect(await screen.findByRole("alert")).toBeInTheDocument();
  });
});

describe("appearance", () => {
  it("changes the reduce motion preference", async () => {
    const handlers = renderPage();
    const user = userEvent.setup();
    await user.click(nav().getByRole("button", { name: "Appearance" }));
    expect(screen.getByRole("button", { name: "Automatic" })).toHaveAttribute(
      "aria-current",
      "true",
    );

    await user.click(screen.getByRole("button", { name: "Enabled" }));

    expect(handlers.onChangeReduceMotion).toHaveBeenCalledWith("on");
  });

  it("still changes the transition effect", async () => {
    const handlers = renderPage();
    const user = userEvent.setup();
    await user.click(nav().getByRole("button", { name: "Appearance" }));

    await user.click(screen.getByRole("button", { name: "Off" }));

    expect(handlers.onChangeLoadingEnabled).toHaveBeenCalledWith(false);
  });
});

describe("notifications", () => {
  it("shows each switch with its state and reports the change", async () => {
    const handlers = renderPage();
    const user = userEvent.setup();
    await user.click(nav().getByRole("button", { name: "Notifications" }));

    expect(
      screen.getByRole("switch", { name: "Run finished" }),
    ).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("switch", { name: "Run failed" })).toHaveAttribute(
      "aria-checked",
      "false",
    );

    await user.click(screen.getByRole("switch", { name: "Run failed" }));
    await user.click(screen.getByRole("switch", { name: "Sound" }));

    expect(handlers.onChangeNotification).toHaveBeenCalledWith(
      "run_failed",
      true,
    );
    expect(handlers.onChangeNotification).toHaveBeenCalledWith("sound", true);
  });
});

describe("advanced", () => {
  it("opens the data folder through the core", async () => {
    core({ open_data_folder: () => undefined });
    renderPage();
    const user = userEvent.setup();
    await user.click(nav().getByRole("button", { name: "Advanced" }));

    expect(screen.getByText("Provider keys")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Open folder" }));

    expect(invokeMock).toHaveBeenCalledWith("open_data_folder");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("shows a translated error when the folder cannot be opened", async () => {
    core({
      open_data_folder: () => {
        throw "data_folder_unavailable";
      },
    });
    renderPage();
    const user = userEvent.setup();
    await user.click(nav().getByRole("button", { name: "Advanced" }));

    await user.click(screen.getByRole("button", { name: "Open folder" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Meelting could not open the data folder.",
    );
  });
});
