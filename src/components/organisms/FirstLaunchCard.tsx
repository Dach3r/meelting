import { Building2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Locale } from "../../i18n/translate";
import { Button } from "../atoms/Button";
import { Logo } from "../atoms/Logo";
import { Alert } from "../molecules/Alert";
import { FormField } from "../molecules/FormField";
import { LocaleSwitcher } from "../molecules/LocaleSwitcher";

type Props = {
  locale: Locale;
  busy: boolean;
  errorMessage: string | null;
  onLocaleChange: (locale: Locale) => void;
  onSubmit: (workspaceName: string) => void;
};

export function FirstLaunchCard({ locale, busy, errorMessage, onLocaleChange, onSubmit }: Props) {
  const { t } = useI18n();
  const [name, setName] = useState("");

  function submit(event: FormEvent) {
    event.preventDefault();
    onSubmit(name);
  }

  return (
    <main className="grid min-h-dvh place-items-center bg-shell p-6">
      <form onSubmit={submit} className="grid w-full max-w-md gap-6 rounded-3xl bg-rail p-8">
        <div className="flex items-center justify-between">
          <Logo />
          <LocaleSwitcher value={locale} onChange={onLocaleChange} />
        </div>
        <div className="grid gap-1">
          <h1 className="text-2xl font-bold">{t("firstLaunch.title")}</h1>
          <p className="text-muted">{t("firstLaunch.subtitle")}</p>
        </div>
        {errorMessage && <Alert message={errorMessage} />}
        <FormField
          label={t("firstLaunch.nameLabel")}
          value={name}
          onChange={(event) => setName(event.target.value)}
          maxLength={80}
          autoFocus
          autoComplete="organization"
          error={errorMessage}
        />
        <Button type="submit" disabled={busy || name.trim() === ""}>
          <Building2 className="size-5" aria-hidden="true" />
          {t("firstLaunch.submit")}
        </Button>
      </form>
    </main>
  );
}
