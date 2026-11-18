import { useLayoutEffect, useRef, useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Team } from "../../hooks/useTeams";
import { AgentDrawer } from "../molecules/AgentDrawer";
import { OrgAgentCard } from "../molecules/OrgAgentCard";
import { OrgCard } from "../molecules/OrgCard";
import { OrgNode } from "../molecules/OrgNode";

const rootId = "root";
const agentTrunkInsetPx = 24;

type Edge = { from: string; to: string; toAgent: boolean };

function relativeBox(container: HTMLElement, element: HTMLElement) {
  const base = container.getBoundingClientRect();
  const box = element.getBoundingClientRect();
  const scale = base.width / container.offsetWidth || 1;
  return {
    left: (box.left - base.left) / scale,
    top: (box.top - base.top) / scale,
    width: box.width / scale,
    height: box.height / scale,
  };
}

function edgePath(container: HTMLElement, cards: Map<string, HTMLElement>, edge: Edge) {
  const fromElement = cards.get(edge.from);
  const toElement = cards.get(edge.to);
  if (!fromElement || !toElement) return null;
  const from = relativeBox(container, fromElement);
  const to = relativeBox(container, toElement);
  const startY = from.top + from.height;
  if (edge.toAgent) {
    const startX = from.left + agentTrunkInsetPx;
    const endY = to.top + to.height / 2;
    return `M ${startX} ${startY} C ${startX} ${endY} ${startX} ${endY} ${to.left} ${endY}`;
  }
  const startX = from.left + from.width / 2;
  const endX = to.left + to.width / 2;
  const middleY = (startY + to.top) / 2;
  return `M ${startX} ${startY} C ${startX} ${middleY} ${endX} ${middleY} ${endX} ${to.top}`;
}

type Props = {
  teams: Team[];
  zoom: number;
  onNewTeam: () => void;
  onNewAgent: (teamId: string) => void;
  onEditAgent: (agentId: string) => void;
  onDeleteAgent: (agentId: string) => void;
  onMoveNode: (nodeId: string, x: number, y: number) => void;
};

export function OrgChart({ teams, zoom, onNewTeam, onNewAgent, onEditAgent, onDeleteAgent, onMoveNode }: Props) {
  const { t } = useI18n();
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(new Set());
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [paths, setPaths] = useState<string[]>([]);
  const content = useRef<HTMLDivElement>(null);
  const cards = useRef(new Map<string, HTMLElement>());
  function registerCard(id: string) {
    return (element: HTMLElement | null) => {
      if (element) cards.current.set(id, element);
      else cards.current.delete(id);
    };
  }
  function selection(id: string) {
    return { selected: selectedId === id, onSelect: () => setSelectedId(selectedId === id ? null : id) };
  }

  const drawerTeam = teams.find((team) => team.agents.some((agent) => agent.id === selectedId));
  const drawerAgent = drawerTeam?.agents.find((agent) => agent.id === selectedId);

  const edges: Edge[] = [];
  if (!collapsed.has(rootId)) {
    for (const team of teams) {
      edges.push({ from: rootId, to: team.id, toAgent: false });
      if (!collapsed.has(team.id)) {
        for (const agent of team.agents) edges.push({ from: team.id, to: agent.id, toAgent: true });
      }
    }
  }

  useLayoutEffect(() => {
    const container = content.current;
    if (!container) return;
    const next = edges.flatMap((edge) => edgePath(container, cards.current, edge) ?? []);
    setPaths((current) => (current.join("|") === next.join("|") ? current : next));
  });

  function moveWithinBoard(id: string, current: { x: number; y: number }, x: number, y: number) {
    const container = content.current;
    const card = cards.current.get(id);
    if (container && card && container.offsetWidth > 0) {
      const box = relativeBox(container, card);
      const left = box.left + x - current.x;
      const top = box.top + y - current.y;
      x += Math.min(Math.max(left, 0), Math.max(0, container.offsetWidth - box.width)) - left;
      y += Math.min(Math.max(top, 0), Math.max(0, container.offsetHeight - box.height)) - top;
    }
    onMoveNode(id, x, y);
  }

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
        <div ref={content} style={{ zoom }} className="relative m-auto flex w-max flex-col items-center px-16 pb-40 pt-20">
          <svg className="pointer-events-none absolute left-0 top-0 overflow-visible" width="1" height="1" aria-hidden="true">
            {paths.map((path) => (
              <path key={path} d={path} fill="none" stroke="var(--color-line)" strokeWidth="2" />
            ))}
          </svg>
          <div ref={registerCard(rootId)}>
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
          </div>
          {!collapsed.has(rootId) && (
            <ul className="org-children">
              {teams.map((team) => (
                <OrgNode
                  key={team.id}
                  className="org-branch"
                  x={team.x}
                  y={team.y}
                  zoom={zoom}
                  onMove={(x, y) => moveWithinBoard(team.id, team, x, y)}
                  registerCard={registerCard(team.id)}
                  card={
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
                  }
                >
                  {!collapsed.has(team.id) && (
                    <ul className="org-stack">
                      {team.agents.map((agent) => (
                        <OrgNode
                          key={agent.id}
                          className="org-leaf"
                          x={agent.x}
                          y={agent.y}
                          zoom={zoom}
                          onMove={(x, y) => moveWithinBoard(agent.id, agent, x, y)}
                          registerCard={registerCard(agent.id)}
                          card={
                            <OrgAgentCard
                              seed={agent.id}
                              name={agent.name}
                              role={agent.role}
                              {...selection(agent.id)}
                            />
                          }
                        />
                      ))}
                    </ul>
                  )}
                </OrgNode>
              ))}
            </ul>
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
          onDelete={() => onDeleteAgent(drawerAgent.id)}
          onClose={() => setSelectedId(null)}
        />
      )}
    </>
  );
}
