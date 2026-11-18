import { useRef, useState, type MouseEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Team } from "../../hooks/useTeams";
import type { BoardTool } from "../molecules/BoardTools";
import { OrgAgentCard } from "../molecules/OrgAgentCard";
import { OrgCard } from "../molecules/OrgCard";

const rootId = "root";

type Props = {
  teams: Team[];
  zoom: number;
  tool: BoardTool;
  onNewTeam: () => void;
  onNewAgent: (teamId: string) => void;
};

export function OrgChart({ teams, zoom, tool, onNewTeam, onNewAgent }: Props) {
  const { t } = useI18n();
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(new Set());
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [panning, setPanning] = useState(false);
  const viewport = useRef<HTMLDivElement>(null);
  const grab = useRef<{ x: number; y: number; left: number; top: number } | null>(null);
  const selectable = tool === "select";

  function startPan(event: MouseEvent<HTMLDivElement>) {
    if (tool !== "pan" || !viewport.current) return;
    const { scrollLeft, scrollTop } = viewport.current;
    grab.current = { x: event.clientX, y: event.clientY, left: scrollLeft, top: scrollTop };
    setPanning(true);
  }

  function movePan(event: MouseEvent<HTMLDivElement>) {
    if (!grab.current || !viewport.current) return;
    viewport.current.scrollLeft = grab.current.left - (event.clientX - grab.current.x);
    viewport.current.scrollTop = grab.current.top - (event.clientY - grab.current.y);
  }

  function endPan() {
    grab.current = null;
    setPanning(false);
  }

  function selection(id: string) {
    return selectable
      ? { selected: selectedId === id, onSelect: () => setSelectedId(selectedId === id ? null : id) }
      : {};
  }

  function toggle(id: string) {
    const next = new Set(collapsed);
    if (!next.delete(id)) next.add(id);
    setCollapsed(next);
  }

  return (
    <div
      ref={viewport}
      data-testid="board"
      onClick={() => setSelectedId(null)}
      onMouseDown={startPan}
      onMouseMove={movePan}
      onMouseUp={endPan}
      onMouseLeave={endPan}
      className={`absolute inset-0 overflow-auto ${tool === "pan" ? (panning ? "cursor-grabbing" : "cursor-grab") : ""}`}
    >
      <div style={{ zoom }} className="mx-auto flex w-max flex-col items-center px-16 py-20">
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
                          <OrgAgentCard seed={agent.id} name={agent.name} role={agent.role} {...selection(agent.id)} />
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
