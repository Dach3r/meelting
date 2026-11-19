import { ArrowDown, Check } from "lucide-react";
import { useEffect, useId, useState, type KeyboardEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Team } from "../../hooks/useTeams";
import { Avatar } from "../atoms/Avatar";

type Props = {
  teams: Team[];
  currentId: string;
  value: string;
  onChange: (teamId: string) => void;
};

const eyebrow = "text-xs font-semibold tracking-wider text-muted uppercase";

export function TeamPicker({ teams, currentId, value, onChange }: Props) {
  const { t } = useI18n();
  const id = useId();
  const sorted = [...teams].sort((a, b) => a.name.localeCompare(b.name));
  const [active, setActive] = useState(() =>
    Math.max(
      sorted.findIndex((team) => team.id === value),
      0,
    ),
  );
  const current = teams.find((team) => team.id === currentId);

  useEffect(() => {
    document
      .getElementById(`${id}-${active}`)
      ?.scrollIntoView({ block: "nearest" });
  }, [active, id]);

  function onKeyDown(event: KeyboardEvent) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      setActive(
        (position) => (position + step + sorted.length) % sorted.length,
      );
    } else if (event.key === "Enter") {
      event.preventDefault();
      onChange(sorted[active].id);
    }
  }

  function agentsAvailable(count: number) {
    if (count === 0) return t("teams.agent.teamAgentsNone");
    if (count === 1) return t("teams.agent.teamAgentsOne");
    return `${t("teams.agent.teamAgentsLead")} ${count} ${t("teams.agent.teamAgentsMany")}`;
  }

  return (
    <div className="flex flex-1 flex-col gap-3">
      {current && (
        <div className="flex items-center gap-4 rounded-2xl border border-line bg-shell p-4">
          <Avatar
            seed={current.name}
            label={current.name}
            className="size-10 rounded-xl text-base"
          />
          <div className="min-w-0">
            <p className="truncate text-sm font-semibold text-ink">
              {current.name}
            </p>
            <p className="text-xs text-muted">
              {agentsAvailable(current.agents.length)}
            </p>
          </div>
        </div>
      )}
      <ArrowDown
        aria-hidden
        className="size-5 shrink-0 self-center text-muted"
      />
      <div className="flex min-h-56 min-w-0 flex-1 flex-col overflow-hidden rounded-2xl border border-line">
        <div
          className={`flex items-center gap-3 border-b border-line bg-shell px-3.5 py-2 ${eyebrow}`}
        >
          <span className="flex-1 pl-7">{t("teams.agent.name")}</span>
          <span className="w-16 text-right">{t("teams.chart.agents")}</span>
        </div>
        <div
          role="listbox"
          tabIndex={0}
          aria-label={t("teams.agent.team")}
          aria-activedescendant={`${id}-${active}`}
          onKeyDown={onKeyDown}
          className="flex-1 overflow-y-auto overscroll-contain focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ink"
        >
          {sorted.map((team, position) => {
            const selected = team.id === value;
            return (
              <div
                key={team.id}
                id={`${id}-${position}`}
                role="option"
                aria-selected={selected}
                onMouseEnter={() => setActive(position)}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => onChange(team.id)}
                className={`flex cursor-pointer items-center gap-3 border-b border-line/60 px-3.5 py-2.5 text-sm text-ink ${selected ? "bg-hi font-semibold" : position === active ? "bg-hi/50" : ""}`}
              >
                <span
                  aria-hidden
                  className={`grid size-4 shrink-0 place-items-center rounded-full border-[1.5px] ${selected ? "border-accent bg-accent text-surface" : "border-muted"}`}
                >
                  {selected && <Check className="size-2.5" strokeWidth={3.5} />}
                </span>
                <span className="flex-1 truncate">{team.name}</span>
                {team.id === currentId && (
                  <span className="rounded-full bg-primary px-2 py-0.5 text-[10px] font-semibold text-accent">
                    {t("teams.agent.teamCurrentTag")}
                  </span>
                )}
                <span className="w-16 text-right font-normal tabular-nums text-muted">
                  {team.agents.length}
                </span>
              </div>
            );
          })}
        </div>
        <p className="border-t border-line bg-shell px-3.5 py-2 text-xs text-muted">
          {teams.length} {t("teams.agent.teamCountUnit")}
        </p>
      </div>
    </div>
  );
}
