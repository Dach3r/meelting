import { createContext, useContext, type ReactNode } from "react";
import { translate, type Locale, type TimeFormat } from "./translate";

type I18n = {
  locale: Locale;
  timeFormat: TimeFormat;
  t: (key: string) => string;
};

const I18nContext = createContext<I18n | null>(null);

export function I18nProvider({
  locale,
  timeFormat = "auto",
  children,
}: {
  locale: Locale;
  timeFormat?: TimeFormat;
  children: ReactNode;
}) {
  const value = { locale, timeFormat, t: (key: string) => translate(locale, key) };
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18n {
  const context = useContext(I18nContext);
  if (!context) throw new Error("useI18n needs an I18nProvider");
  return context;
}
