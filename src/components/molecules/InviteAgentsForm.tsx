import { useState, type FormEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Agent } from "../../hooks/useTeams";
import { Button } from "../atoms/Button";
import { AgentPicker } from "./AgentPicker";

type Props = {
  agents: Agent[];
  busy: boolean;
  onSubmit: (agentIds: string[]) => void;
  onCancel: () => void;
};

export function InviteAgentsForm({ agents, busy, onSubmit, onCancel }: Props) {
  const { t } = useI18n();
  const [agentIds, setAgentIds] = useState<string[]>([]);

  function submit(event: FormEvent) {
    event.preventDefault();
    onSubmit(agentIds);
  }

  return (
    <form onSubmit={submit} className="grid gap-5 p-6 sm:px-10 sm:pb-8">
      <AgentPicker agents={agents} value={agentIds} onChange={setAgentIds} />
      <div className="flex justify-end gap-3">
        <Button type="button" variant="ghost" size="pill" onClick={onCancel}>
          {t("rooms.cancel")}
        </Button>
        <Button
          type="submit"
          size="pill"
          disabled={busy || agentIds.length === 0}
        >
          {t("rooms.invite.submit")}
        </Button>
      </div>
    </form>
  );
}
