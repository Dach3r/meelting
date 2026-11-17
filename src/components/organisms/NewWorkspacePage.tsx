import { ArrowRight } from "lucide-react";
import { useId, useState, type FormEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Locale } from "../../i18n/translate";
import { Button } from "../atoms/Button";
import { Logo } from "../atoms/Logo";
import { Alert } from "../molecules/Alert";
import { LocaleSwitcher } from "../molecules/LocaleSwitcher";

type Props = {
  busy: boolean;
  errorMessage: string | null;
  onSubmit: (workspaceName: string) => void;
  language?: { locale: Locale; onChange: (locale: Locale) => void };
  onCancel?: () => void;
};

export function NewWorkspacePage({ busy, errorMessage, onSubmit, language, onCancel }: Props) {
  const { t } = useI18n();
  const inputId = useId();
  const [name, setName] = useState("");

  function submit(event: FormEvent) {
    event.preventDefault();
    onSubmit(name);
  }

  return (
    <main className="grid min-h-dvh grid-rows-[auto_1fr] bg-shell px-6 py-6 sm:px-10">
      <header className="flex items-center justify-between">
        <Logo />
        {language && <LocaleSwitcher value={language.locale} onChange={language.onChange} />}
        {onCancel && (
          <Button type="button" variant="ghost" size="compact" onClick={onCancel}>
            {t("newWorkspace.cancel")}
          </Button>
        )}
      </header>
      <form onSubmit={submit} className="mx-auto grid w-full max-w-3xl content-center gap-4 pb-16">
        <h1 className="text-lg font-medium text-muted">{t("newWorkspace.title")}</h1>
        <label htmlFor={inputId} className="sr-only">
          {t("newWorkspace.nameLabel")}
        </label>
        <input
          id={inputId}
          value={name}
          onChange={(event) => setName(event.target.value)}
          maxLength={80}
          autoFocus
          autoComplete="organization"
          aria-invalid={errorMessage ? true : undefined}
          placeholder={t("newWorkspace.placeholder")}
          className={`w-full min-w-0 border-0 border-b-2 bg-transparent pb-4 text-4xl font-bold tracking-tight text-ink outline-none transition-colors placeholder:text-line sm:text-6xl ${errorMessage ? "border-error" : "border-line focus:border-ink"}`}
        />
        {errorMessage && <Alert message={errorMessage} />}
        <div className="flex items-center justify-between gap-6">
          <p className="text-muted">{t("newWorkspace.subtitle")}</p>
          <button
            type="submit"
            disabled={busy || name.trim() === ""}
            aria-label={t("newWorkspace.submit")}
            className="grid size-16 shrink-0 cursor-pointer place-items-center rounded-full bg-ink text-rail transition duration-200 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink enabled:hover:translate-x-1 disabled:cursor-not-allowed disabled:opacity-30"
          >
            <ArrowRight className="size-7" aria-hidden="true" />
          </button>
        </div>
      </form>
    </main>
  );
}
