import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Team } from "../../hooks/useTeams";
import { AgentDrawer } from "../molecules/AgentDrawer";
import { OrgAgentCard } from "../molecules/OrgAgentCard";
import { OrgCard } from "../molecules/OrgCard";

const rootId = "root";

type Props = {
  teams: Team[];
  zoom: number;
  onNewTeam: () => void;
  onNewAgent: (teamId: string) => void;
  onEditAgent: (agentId: string) => void;
};

export function OrgChart({ teams, zoom, onNewTeam, onNewAgent, onEditAgent }: Props) {
  const { t } = useI18n();
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(new Set());
  const [selectedId, setSelectedId] = useState<string | null>(null);
  function selection(id: string) {
    return { selected: selectedId === id, onSelect: () => setSelectedId(selectedId === id ? null : id) };
  }

  const drawerTeam = teams.find((team) => team.agents.some((agent) => agent.id === selectedId));
  const drawerAgent = drawerTeam?.agents.find((agent) => agent.id === selectedId);

  function toggle(id: string) {
    const next = new Set(collapsed);
    if (!next.delete(id)) next.add(id);
    setCollapsed(next);
  }

  return (
    <>
      <div
        data-testid="board"
        onClick={() => !drawerAgent && setSelectedId(null)}
        className="absolute inset-0 flex overflow-auto"
      >
        <div style={{ zoom }} className="m-auto flex w-max flex-col items-center px-16 pb-40 pt-20">
          <OrgCard
            seed={rootId}
            name={t("teams.chart.you")}
            role={t("teams.chart.supervisor")}
            count={teams.length}
            expanded={!collapsed.has(rootId)}
            onToggle={() => toggle(rootId)}
            addLabel={t("teams.newTeam")}
            onAdd={onNewTeam}
            {...selection(rootId)}
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
                      addLabel={t("teams.hireAgent")}
                      onAdd={() => onNewAgent(team.id)}
                      {...selection(team.id)}
                    />
                    {!collapsed.has(team.id) && (
                      <ul className="org-stack">
                        {team.agents.map((agent) => (
                          <li key={agent.id} className="org-leaf">
                            <OrgAgentCard
                              seed={agent.id}
                              name={agent.name}
                              role={agent.role}
                              {...selection(agent.id)}
                            />
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
      {drawerTeam && drawerAgent && (
        <AgentDrawer
          seed={drawerAgent.id}
          name={drawerAgent.name}
          role={drawerAgent.role}
          teamName={drawerTeam.name}
          onEdit={() => onEditAgent(drawerAgent.id)}
          onClose={() => setSelectedId(null)}
        />
      )}
    </>
  );
}
