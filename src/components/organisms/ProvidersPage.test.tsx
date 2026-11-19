import { invoke } from "@tauri-apps/api/core";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProviderKeyStatus } from "../../hooks/useProviderKeys";
import { I18nProvider } from "../../i18n/I18nProvider";
import { ProvidersPage } from "./ProvidersPage";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);
const SECRET = "sk-super-secret-value-4321";

type Handlers = Record<string, (args: Record<string, unknown>) => unknown>;

function status(
  provider: string,
  name: string,
  overrides: Partial<ProviderKeyStatus> = {},
): ProviderKeyStatus {
  return { provider, name, last_four: null, offline: false, ...overrides };
}

let statuses: ProviderKeyStatus[];

function mockCore(handlers: Handlers = {}) {
  const all: Handlers = {
    list_provider_keys: () => statuses,
    list_llm_providers: () => [
      { key: "anthropic", name: "Anthropic", location: "cloud", models: [] },
      { key: "ollama", name: "Ollama", location: "local", models: [] },
      { key: "lmstudio", name: "LM Studio", location: "local", models: [] },
    ],
    detect_local_models: () => [
      { key: "ollama", running: true, models: ["qwen", "llama"] },
      { key: "lmstudio", running: false, models: [] },
    ],
    ...handlers,
  };
  invokeMock.mockImplementation(async (command, args) => {
    const handler = all[command];
    if (!handler) throw new Error(`unexpected ${command}`);
    return handler((args ?? {}) as Record<string, unknown>);
  });
}

function renderPage() {
  return render(
    <I18nProvider locale="en">
      <ProvidersPage />
    </I18nProvider>,
  );
}

function row(name: string) {
  return screen.getByRole("heading", { name }).closest("div.flex-col")!;
}

async function typeKey(name: string, key: string) {
  const user = userEvent.setup();
  const scope = within(row(name) as HTMLElement);
  await user.click(scope.getByRole("button", { name: /add key|replace/i }));
  const field = scope.getByLabelText(`${name} API key`);
  await user.type(field, key);
  await user.click(scope.getByRole("button", { name: "Verify and save" }));
  return { user, scope, field };
}

beforeEach(() => {
  invokeMock.mockReset();
  statuses = [
    status("anthropic", "Anthropic"),
    status("openai", "OpenAI", { last_four: "abcd" }),
    status("groq", "Groq", { last_four: "wxyz", offline: true }),
  ];
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("ProvidersPage", () => {
  it("shows configured, missing and offline cloud keys and the local runtimes", async () => {
    mockCore();
    renderPage();

    expect(await screen.findByText("2 of 3 configured")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 1, name: "AI providers" }),
    ).toBeInTheDocument();
    expect(
      within(row("Anthropic") as HTMLElement).getByText("Not configured"),
    ).toBeInTheDocument();
    expect(screen.getByText("Configured · ····abcd")).toBeInTheDocument();
    expect(screen.getByText("No internet connection")).toBeInTheDocument();
    expect(await screen.findByText("Running · 2 models")).toBeInTheDocument();
    expect(screen.getByText("Not running")).toBeInTheDocument();
  });

  it("verifies a new key, saves it and clears the field", async () => {
    const saved = vi.fn(() => {
      statuses[0] = { ...statuses[0], last_four: "4321" };
      return { verification: { status: "verified" }, last_four: "4321" };
    });
    mockCore({ save_provider_key: saved });
    renderPage();
    await screen.findByText("2 of 3 configured");

    await typeKey("Anthropic", SECRET);

    expect(saved).toHaveBeenCalledWith({ provider: "anthropic", key: SECRET });
    expect(await screen.findByText("Connected · ····4321")).toBeInTheDocument();
    expect(
      screen.queryByLabelText("Anthropic API key"),
    ).not.toBeInTheDocument();
  });

  it("uses a password field that the browser does not remember or correct", async () => {
    mockCore();
    renderPage();
    await screen.findByText("2 of 3 configured");
    const user = userEvent.setup();

    await user.click(
      within(row("Anthropic") as HTMLElement).getByRole("button", {
        name: "Add key",
      }),
    );
    const field = screen.getByLabelText("Anthropic API key");

    expect(field).toHaveAttribute("type", "password");
    expect(field).toHaveAttribute("autocomplete", "off");
    expect(field).toHaveAttribute("spellcheck", "false");
  });

  it("keeps the previous key when a replacement is rejected and never shows the key", async () => {
    mockCore({
      save_provider_key: () => ({
        verification: { status: "rejected", reason: "provider_auth_failed" },
        last_four: "abcd",
      }),
    });
    const setItem = vi.spyOn(Storage.prototype, "setItem");
    renderPage();
    await screen.findByText("2 of 3 configured");

    const { field } = await typeKey("OpenAI", SECRET);

    expect(
      await screen.findByText(
        "OpenAI rejected the key. Check that it is complete and active. The previous key (····abcd) is kept.",
      ),
    ).toBeInTheDocument();
    expect(field).toHaveValue("");
    expect(document.body.textContent).not.toContain(SECRET);
    expect(
      setItem.mock.calls.flat().some((value) => String(value).includes(SECRET)),
    ).toBe(false);
  });

  it("tells an offline verification apart from a busy provider", async () => {
    let outcome = "offline";
    mockCore({
      save_provider_key: () => ({
        verification: { status: outcome },
        last_four: null,
      }),
    });
    renderPage();
    await screen.findByText("2 of 3 configured");

    const { user, scope } = await typeKey("Anthropic", SECRET);
    expect(
      await screen.findByText(
        "Could not verify because Anthropic could not be reached. Check your internet connection. The key was not saved.",
      ),
    ).toBeInTheDocument();

    outcome = "provider_busy";
    await user.type(scope.getByLabelText("Anthropic API key"), SECRET);
    await user.click(scope.getByRole("button", { name: "Verify and save" }));
    expect(
      await screen.findByText(
        "Could not verify because Anthropic is not responding right now. The key was not saved; try again in a moment.",
      ),
    ).toBeInTheDocument();
  });

  it("shows a translated core error without the key", async () => {
    mockCore({
      save_provider_key: () => {
        throw "keychain_unavailable";
      },
    });
    renderPage();
    await screen.findByText("2 of 3 configured");

    await typeKey("Anthropic", SECRET);

    expect(
      await screen.findByText(
        "Could not access the system keychain. Unlock it and try again.",
      ),
    ).toBeInTheDocument();
    expect(document.body.textContent).not.toContain(SECRET);
  });

  it("checks a stored key and reports a rejection without deleting it", async () => {
    const check = vi.fn(() => ({
      verification: { status: "rejected", reason: "provider_billing" },
      last_four: "abcd",
    }));
    mockCore({ check_provider_key: check });
    renderPage();
    await screen.findByText("2 of 3 configured");
    const user = userEvent.setup();

    await user.click(
      within(row("OpenAI") as HTMLElement).getByRole("button", {
        name: "Check connection",
      }),
    );

    expect(check).toHaveBeenCalledWith({ provider: "openai" });
    expect(await screen.findByText("Key rejected")).toBeInTheDocument();
    expect(
      screen.getByText(
        "OpenAI accepted the key, but the account has no balance or quota left.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("2 of 3 configured")).toBeInTheDocument();
  });

  it("asks for confirmation before deleting a key", async () => {
    const deleted = vi.fn(() =>
      statuses.map((entry) =>
        entry.provider === "openai" ? { ...entry, last_four: null } : entry,
      ),
    );
    mockCore({ delete_provider_key: deleted });
    renderPage();
    await screen.findByText("2 of 3 configured");
    const user = userEvent.setup();

    await user.click(
      within(row("OpenAI") as HTMLElement).getByRole("button", {
        name: "Delete",
      }),
    );
    const dialog = screen.getByRole("dialog", {
      name: "Delete the OpenAI key?",
    });
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(deleted).not.toHaveBeenCalled();

    await user.click(
      within(row("OpenAI") as HTMLElement).getByRole("button", {
        name: "Delete",
      }),
    );
    await user.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: "Delete key",
      }),
    );

    expect(deleted).toHaveBeenCalledWith({ provider: "openai" });
    await waitFor(() =>
      expect(screen.getByText("1 of 3 configured")).toBeInTheDocument(),
    );
  });
});
