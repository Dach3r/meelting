import en from "./en.json";
import es from "./es.json";

export type Locale = "es" | "en";

const messages: Record<Locale, unknown> = { es, en };

export function translate(locale: Locale, key: string): string {
  const value = key
    .split(".")
    .reduce<unknown>(
      (node, part) => (node as Record<string, unknown> | undefined)?.[part],
      messages[locale],
    );
  return typeof value === "string" ? value : key;
}

export function translateError(locale: Locale, code: string): string {
  const key = `errors.${code}`;
  const text = translate(locale, key);
  return text === key ? translate(locale, "errors.unknown") : text;
}

export function formatDateTime(locale: Locale, iso: string): string {
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(new Date(iso));
}

export function formatAgentCount(locale: Locale, count: number): string {
  if (count === 0) return translate(locale, "rooms.agentsNone");
  if (count === 1) return translate(locale, "rooms.agentsOne");
  return translate(locale, "rooms.agentsMany").replace("{count}", String(count));
}

export function translateWith(locale: Locale, key: string, values: Record<string, string | number>): string {
  return Object.entries(values).reduce(
    (text, [name, value]) => text.split(`{${name}}`).join(String(value)),
    translate(locale, key),
  );
}

export function translateRunError(locale: Locale, errorKind: string | null, provider: string, local: boolean): string {
  const kind = errorKind === "provider_unavailable" && local ? "provider_unavailable_local" : errorKind;
  const key = `agentRun.errors.${kind}`;
  const known = translate(locale, key) !== key;
  return translateWith(locale, known ? key : "agentRun.errors.unknown", { provider });
}
