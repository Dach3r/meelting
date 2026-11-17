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
