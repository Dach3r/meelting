import { Plus } from "lucide-react";
import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Team } from "../../hooks/useTeams";
import { Button } from "../atoms/Button";
import { OrgAgentCard } from "../molecules/OrgAgentCard";
import { OrgCard } from "../molecules/OrgCard";

const rootId = "root";

type Props = {
  teams: Team[];
  zoom: number;
  onNewAgent: (teamId: string) => void;
};

export function OrgChart({ teams, zoom, onNewAgent }: Props) {
  const { t } = useI18n();
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(new Set());

  function toggle(id: string) {
    const next = new Set(collapsed);
    if (!next.delete(id)) next.add(id);
    setCollapsed(next);
  }

  return (
    <div className="overflow-x-auto pb-4">
      <div style={{ zoom }} className="mx-auto flex w-max flex-col items-center">
        <OrgCard
          seed={rootId}
          name={t("teams.chart.you")}
          role={t("teams.chart.supervisor")}
          count={teams.length}
          expanded={!collapsed.has(rootId)}
          onToggle={() => toggle(rootId)}
        />
        {!collapsed.has(rootId) && (
          <>
            <span className="org-stem" />
            <ul className="org-children">
              {teams.map((team) => (
                <li key={team.id} className="org-branch">
                  <OrgCard
                    seed={team.id}
                    name={team.name}
                    role={t("teams.chart.team")}
                    asHeading
                    count={team.agents.length}
                    expanded={!collapsed.has(team.id)}
                    onToggle={() => toggle(team.id)}
                  />
                  {team.agents.length === 0 && <p className="mt-2 w-49 text-center text-sm text-muted">{t("teams.noAgents")}</p>}
                  {!collapsed.has(team.id) && (
                    <ul className="org-stack">
                      {team.agents.map((agent) => (
                        <li key={agent.id} className="org-leaf">
                          <OrgAgentCard seed={agent.id} name={agent.name} role={agent.role} />
                        </li>
                      ))}
                      <li className="org-leaf">
                        <Button
                          type="button"
                          variant="ghost"
                          size="compact"
                          className="h-14 w-40 border-dashed border-line!"
                          onClick={() => onNewAgent(team.id)}
                        >
                          <Plus className="size-4" aria-hidden="true" />
                          {t("teams.newAgent")}
                        </Button>
                      </li>
                    </ul>
                  )}
                </li>
              ))}
            </ul>
          </>
        )}
      </div>
    </div>
  );
}
