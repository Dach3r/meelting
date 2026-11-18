import { ArrowRight, Check, Info, Search } from "lucide-react";
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
  const [query, setQuery] = useState("");
  const sorted = [...teams].sort((a, b) => a.name.localeCompare(b.name));
  const visible = sorted.filter((team) => team.name.toLowerCase().includes(query.trim().toLowerCase()));
  const [active, setActive] = useState(() => Math.max(sorted.findIndex((team) => team.id === value), 0));
  const current = teams.find((team) => team.id === currentId);

  useEffect(() => {
    document.getElementById(`${id}-${active}`)?.scrollIntoView({ block: "nearest" });
  }, [active, id]);

  function onKeyDown(event: KeyboardEvent) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      if (visible.length > 0) setActive((position) => (position + step + visible.length) % visible.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (visible[active]) onChange(visible[active].id);
    }
  }

  return (
    <div className="flex flex-1 flex-col gap-6 sm:flex-row">
      <div className="grid w-full content-start gap-3 sm:w-40 sm:shrink-0">
        <p className={eyebrow}>{t("teams.agent.teamCurrent")}</p>
        {current && (
          <div className="flex h-32 flex-col justify-between rounded-2xl border border-line bg-shell p-4">
            <Avatar seed={current.name} label={current.name} className="size-9 rounded-xl text-sm" />
            <div className="min-w-0">
              <p className="truncate text-sm font-semibold text-ink">{current.name}</p>
              <p className="text-xs text-muted">
                {t("teams.chart.agents")}: {current.agents.length}
              </p>
            </div>
          </div>
        )}
        <p className="flex gap-2 text-xs leading-relaxed text-muted">
          <Info aria-hidden className="mt-0.5 size-3.5 shrink-0" />
          {t("teams.agent.teamResetNote")}
        </p>
      </div>
      <ArrowRight aria-hidden className="mt-[4.75rem] hidden size-5 shrink-0 self-start text-muted sm:block" />
      <div className="flex min-w-0 flex-1 flex-col gap-2.5" onKeyDown={onKeyDown}>
        <div className="flex items-center gap-3">
          <span id={`${id}-label`} className={eyebrow}>
            {t("teams.agent.teamMoveTo")}
          </span>
          <div className="relative flex-1">
            <Search aria-hidden className="pointer-events-none absolute top-1/2 left-3 size-3.5 -translate-y-1/2 text-muted" />
            <input
              type="text"
              role="combobox"
              aria-expanded
              aria-controls={`${id}-list`}
              aria-activedescendant={visible.length > 0 ? `${id}-${active}` : undefined}
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
                setActive(0);
              }}
              placeholder={t("teams.agent.teamSearch")}
              aria-label={t("teams.agent.teamSearch")}
              className="w-full rounded-xl border border-line bg-shell py-2 pr-3 pl-9 text-sm text-ink outline-none placeholder:text-muted focus-visible:border-ink"
            />
          </div>
        </div>
        <div className="flex min-h-64 flex-1 flex-col overflow-hidden rounded-2xl border border-line">
          <div className={`flex items-center gap-3 border-b border-line bg-shell px-3.5 py-2 ${eyebrow}`}>
            <span className="flex-1 pl-7">{t("teams.agent.name")}</span>
            <span className="w-16 text-right">{t("teams.chart.agents")}</span>
          </div>
          <div id={`${id}-list`} role="listbox" aria-labelledby={`${id}-label`} className="flex-1 overflow-y-auto overscroll-contain">
            {visible.length === 0 && <p className="px-3 py-4 text-center text-sm text-muted">{t("teams.agent.teamEmpty")}</p>}
            {visible.map((team, position) => {
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
                    <span className="rounded-full bg-primary px-2 py-0.5 text-[10px] font-semibold text-accent">{t("teams.agent.teamCurrentTag")}</span>
                  )}
                  <span className="w-16 text-right font-normal tabular-nums text-muted">{team.agents.length}</span>
                </div>
              );
            })}
          </div>
          <p className="border-t border-line bg-shell px-3.5 py-2 text-xs text-muted">
            {visible.length === teams.length ? teams.length : `${visible.length} / ${teams.length}`} {t("teams.agent.teamCountUnit")}
          </p>
        </div>
      </div>
    </div>
  );
}
