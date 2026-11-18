import { useState, type FormEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { Avatar } from "../atoms/Avatar";
import { Button } from "../atoms/Button";
import { Field } from "../atoms/Field";
import { Alert } from "./Alert";

type Props = {
  busy: boolean;
  errorMessage: string | null;
  onSubmit: (name: string) => Promise<boolean>;
  onCancel: () => void;
};

export function TeamForm({ busy, errorMessage, onSubmit, onCancel }: Props) {
  const { t } = useI18n();
  const [name, setName] = useState("");
  const trimmed = name.trim();

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (await onSubmit(name)) onCancel();
  }

  return (
    <form onSubmit={submit} className="mt-5 grid border-t border-line sm:grid-cols-[11rem_1fr]">
      <div aria-hidden="true" className="flex flex-col items-center justify-center gap-4 border-b border-line bg-raised p-5 sm:border-r sm:border-b-0">
        <div className="flex flex-col items-center">
          <span className="rounded-xl border border-line bg-surface px-3 py-1.5 text-xs font-semibold text-muted">{t("teams.chart.you")}</span>
          <span className="h-4 w-0.5 bg-line" />
          <span className="flex max-w-36 items-center gap-2 rounded-xl border border-line bg-surface p-1.5 pr-3 text-sm font-bold text-ink">
            <Avatar seed={trimmed} label={trimmed || "?"} className="size-6 rounded-full text-xs" />
            <span className="truncate">{trimmed || t("teams.newTeam")}</span>
          </span>
          <span className="h-4 w-0.5 bg-line" />
          <span className="flex gap-2">
            {[0, 1, 2].map((slot) => (
              <span key={slot} className="size-7 rounded-full border-2 border-dashed border-line" />
            ))}
          </span>
        </div>
        <p className="text-center text-xs text-muted">{t("teams.preview")}</p>
      </div>
      <div className="flex flex-col">
        <div className="grid gap-4 px-6 py-5">
          <Field label={t("teams.teamName")} value={name} onChange={setName} maxLength={80} autoFocus />
          {errorMessage && <Alert message={errorMessage} />}
        </div>
        <div className="mt-auto flex justify-end gap-3 px-6 pb-6">
          <Button type="button" variant="ghost" size="pill" onClick={onCancel}>
            {t("teams.cancel")}
          </Button>
          <Button type="submit" size="pill" disabled={busy || trimmed === ""}>
            {t("teams.createTeam")}
          </Button>
        </div>
      </div>
    </form>
  );
}
