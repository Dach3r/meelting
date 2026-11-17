import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Team } from "../../hooks/useTeams";
import { OrgAgentCard } from "../molecules/OrgAgentCard";
import { OrgCard } from "../molecules/OrgCard";

const rootId = "root";

type Props = {
  teams: Team[];
  zoom: number;
  onNewTeam: () => void;
  onNewAgent: (teamId: string) => void;
};

export function OrgChart({ teams, zoom, onNewTeam, onNewAgent }: Props) {
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
          addLabel={t("teams.newTeam")}
          onAdd={onNewTeam}
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
                    addLabel={t("teams.newAgent")}
                    onAdd={() => onNewAgent(team.id)}
                  />
                  {!collapsed.has(team.id) && (
                    <ul className="org-stack">
                      {team.agents.map((agent) => (
                        <li key={agent.id} className="org-leaf">
                          <OrgAgentCard seed={agent.id} name={agent.name} role={agent.role} />
                        </li>
                      ))}
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
