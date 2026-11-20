import { describe, expect, it } from "vitest";
import en from "./en.json";
import es from "./es.json";
import {
  formatAgentCount,
  formatDateTime,
  formatTime,
  translate,
  translateError,
  translateRunError,
  translateWith,
} from "./translate";

function keys(node: unknown, prefix = ""): string[] {
  return Object.entries(node as Record<string, unknown>).flatMap(
    ([key, value]) =>
      typeof value === "string"
        ? [`${prefix}${key}`]
        : keys(value, `${prefix}${key}.`),
  );
}

describe("translate", () => {
  it("returns the text for the locale", () => {
    expect(translate("es", "home.nav.teams")).toBe("Equipos");
    expect(translate("en", "home.nav.teams")).toBe("Teams");
  });

  it("falls back to the key when it is missing", () => {
    expect(translate("en", "missing.key")).toBe("missing.key");
  });

  it("translates known error codes and falls back to a generic message", () => {
    expect(translateError("es", "invalid_name")).toBe(
      "Escribe un nombre de hasta 80 caracteres.",
    );
    expect(translateError("en", "weird")).toBe(
      translate("en", "errors.unknown"),
    );
  });

  it("counts the agents of a room in words", () => {
    expect(formatAgentCount("en", 0)).toBe("No agents");
    expect(formatAgentCount("es", 1)).toBe("1 agente");
    expect(formatAgentCount("en", 3)).toBe("3 agents");
  });

  it("fills in named values", () => {
    expect(translateWith("en", "agentRun.busy", { agent: "Ana" })).toBe(
      "Ana is still working on a request.",
    );
    expect(translateWith("es", "agentRun.retrying", { attempt: 2 })).toBe(
      "Problema temporal. Reintentando (intento 2 de 3)…",
    );
  });

  it("explains every run error with the provider name and says when a local provider is not running", () => {
    expect(
      translateRunError("en", "provider_not_configured", "Groq", false),
    ).toBe("Groq has no API key in this workspace.");
    expect(
      translateRunError("es", "provider_unavailable", "Ollama", true),
    ).toBe("Ollama no está en ejecución. Ábrelo e inténtalo de nuevo.");
    expect(
      translateRunError("en", "provider_unavailable", "OpenAI", false),
    ).toBe("OpenAI isn't available right now.");
    expect(translateRunError("es", "output_truncated", "Ollama", true)).toBe(
      "La respuesta del agente era demasiado larga y no se guardó.",
    );
    expect(translateRunError("en", "something_new", "Anthropic", false)).toBe(
      "The request to Anthropic failed.",
    );
  });

  it("has a text for every error kind the core can report", () => {
    const kinds = [
      "rate_limited",
      "timeout",
      "provider_unavailable",
      "provider_not_configured",
      "model_unavailable",
      "provider_auth_failed",
      "provider_billing",
      "provider_rejected",
      "content_filtered",
      "output_truncated",
      "invalid_response",
      "context_too_large",
      "tools_unsupported",
      "iteration_budget_exhausted",
      "tool_call_budget_exhausted",
      "tool_output_budget_exhausted",
      "cancelled",
      "expired",
    ];
    for (const locale of ["en", "es"] as const) {
      for (const kind of kinds) {
        expect(translate(locale, `agentRun.errors.${kind}`)).not.toBe(
          `agentRun.errors.${kind}`,
        );
      }
      for (const code of [
        "invalid_client_request_id",
        "agent_busy",
        "client_request_conflict",
        "agent_has_activity",
        "team_has_agents",
      ]) {
        expect(translateError(locale, code)).not.toBe(
          translate(locale, "errors.unknown"),
        );
      }
    }
  });

  it("has the same keys in both languages", () => {
    expect(keys(es).sort()).toEqual(keys(en).sort());
  });
});

describe("time format", () => {
  const iso = "2026-11-18T15:30:00.000Z";

  it("keeps the locale default when automatic", () => {
    expect(formatTime("en", iso)).toBe(formatTime("en", iso, "auto"));
  });

  it("forces the 12-hour clock", () => {
    expect(formatTime("es", iso, "12h")).toMatch(/[ap]\.?\s?m/i);
    expect(formatDateTime("es", iso, "12h")).toMatch(/[ap]\.?\s?m/i);
  });

  it("forces the 24-hour clock", () => {
    expect(formatTime("en", iso, "24h")).not.toMatch(/[ap]\.?m/i);
    expect(formatDateTime("en", iso, "24h")).not.toMatch(/[ap]\.?m/i);
  });
});
