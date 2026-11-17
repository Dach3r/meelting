import { useState, type FormEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
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

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (await onSubmit(name)) onCancel();
  }

  return (
    <form onSubmit={submit}>
      <div className="grid gap-4 px-6 py-5">
        <Field label={t("teams.teamName")} value={name} onChange={setName} maxLength={80} autoFocus />
        {errorMessage && <Alert message={errorMessage} />}
      </div>
      <div className="flex justify-end gap-3 px-6 pb-6">
        <Button type="button" variant="ghost" size="pill" onClick={onCancel}>
          {t("teams.cancel")}
        </Button>
        <Button type="submit" size="pill" disabled={busy || name.trim() === ""}>
          {t("teams.createTeam")}
        </Button>
      </div>
    </form>
  );
}
