import { Users } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { Avatar } from "../atoms/Avatar";
import { Button } from "../atoms/Button";
import { Field } from "../atoms/Field";
import { Alert } from "./Alert";

export function slugifyTeamName(raw: string) {
  return raw
    .toLowerCase()
    .split(/[^\p{L}\p{N}]+/u)
    .filter(Boolean)
    .join("-");
}

function typedTeamName(raw: string) {
  return raw
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, "-")
    .replace(/^-/, "");
}

type Props = {
  busy: boolean;
  errorMessage: string | null;
  onSubmit: (name: string) => Promise<boolean>;
  onCancel: () => void;
};

export function TeamForm({ busy, errorMessage, onSubmit, onCancel }: Props) {
  const { t } = useI18n();
  const [name, setName] = useState("");
  const slug = slugifyTeamName(name);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (await onSubmit(slug)) onCancel();
  }

  return (
    <form
      onSubmit={submit}
      className="mt-5 grid border-t border-line sm:grid-cols-[15rem_1fr]"
    >
      <div
        aria-hidden="true"
        className="flex flex-col items-center justify-center gap-5 border-b border-line bg-raised/50 px-5 py-8 sm:border-r sm:border-b-0"
      >
        <div className="flex flex-col items-center">
          <span className="rounded-xl border border-line bg-raised px-4 py-2 text-sm font-semibold text-muted">
            {t("teams.chart.you")}
          </span>
          <span className="h-4 w-0.5 bg-line" />
          <span className="flex max-w-full items-center gap-2.5 rounded-2xl border border-line bg-raised p-2 pr-4 text-base font-bold text-ink">
            <Avatar
              seed={slug}
              label={slug || "?"}
              className="size-8 rounded-full text-sm"
            />
            <span className={`truncate ${slug ? "" : "text-muted"}`}>
              {slug || slugifyTeamName(t("teams.newTeam"))}
            </span>
          </span>
          <span className="h-4 w-0.5 bg-line" />
          <span className="flex gap-2">
            {[0, 1, 2].map((slot) => (
              <span
                key={slot}
                className="size-8 rounded-full border-2 border-dashed border-line"
              />
            ))}
          </span>
        </div>
        <p className="text-center text-xs text-muted">{t("teams.preview")}</p>
      </div>
      <div className="flex flex-col">
        <div className="grid gap-4 p-6 sm:px-10 sm:pt-10">
          <Field
            label={t("teams.teamName")}
            value={name}
            onChange={(value) => setName(typedTeamName(value))}
            maxLength={80}
            icon={Users}
            autoFocus
          />
          {errorMessage && <Alert message={errorMessage} />}
        </div>
        <div className="mt-auto flex justify-end gap-3 px-6 pb-6 sm:px-10">
          <Button type="button" variant="ghost" size="pill" onClick={onCancel}>
            {t("teams.cancel")}
          </Button>
          <Button type="submit" size="pill" disabled={busy || slug === ""}>
            {t("teams.createTeam")}
          </Button>
        </div>
      </div>
    </form>
  );
}
