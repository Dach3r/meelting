import { describe, expect, it } from "vitest";
import en from "./en.json";
import es from "./es.json";
import { translate, translateError } from "./translate";

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
    expect(translateError("en", "weird")).toBe(translate("en", "errors.unknown"));
  });

  it("has the same keys in both languages", () => {
    expect(keys(es).sort()).toEqual(keys(en).sort());
  });
});
