import { Video } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Agent } from "../../hooks/useTeams";
import { Button } from "../atoms/Button";
import { Field } from "../atoms/Field";
import { AgentPicker } from "./AgentPicker";
import { Alert } from "./Alert";

type Props = {
  agents: Agent[];
  busy: boolean;
  errorMessage: string | null;
  onSubmit: (name: string, agentIds: string[]) => Promise<void>;
  onCancel: () => void;
};

export function RoomForm({ agents, busy, errorMessage, onSubmit, onCancel }: Props) {
  const { t } = useI18n();
  const [name, setName] = useState("");
  const [agentIds, setAgentIds] = useState<string[]>([]);

  function submit(event: FormEvent) {
    event.preventDefault();
    onSubmit(name, agentIds);
  }

  return (
    <form onSubmit={submit} className="grid gap-5 p-6 sm:px-10 sm:pb-8">
      <Field label={t("rooms.form.name")} value={name} onChange={setName} maxLength={80} icon={Video} autoFocus />
      <AgentPicker agents={agents} value={agentIds} onChange={setAgentIds} />
      {errorMessage && <Alert message={errorMessage} />}
      <div className="flex justify-end gap-3">
        <Button type="button" variant="ghost" size="pill" onClick={onCancel}>
          {t("rooms.cancel")}
        </Button>
        <Button type="submit" size="pill" disabled={busy || name.trim() === ""}>
          {t("rooms.form.submit")}
        </Button>
      </div>
    </form>
  );
}
