import { Search, X } from "lucide-react";
import { useId, useState, type KeyboardEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Agent } from "../../hooks/useTeams";
import { Avatar } from "../atoms/Avatar";
import { fieldClasses } from "../atoms/Field";

type Props = {
  agents: Agent[];
  value: string[];
  onChange: (agentIds: string[]) => void;
  hint?: string;
};

export function AgentPicker({ agents, value, onChange, hint }: Props) {
  const { t } = useI18n();
  const id = useId();
  const [query, setQuery] = useState("");
  const selected = value.flatMap((agentId) =>
    agents.filter((agent) => agent.id === agentId),
  );
  const needle = query.trim().toLowerCase();
  const options = agents.filter(
    (agent) =>
      !value.includes(agent.id) &&
      (agent.name.toLowerCase().includes(needle) ||
        agent.role.toLowerCase().includes(needle)),
  );

  function add(agentId: string) {
    onChange([...value, agentId]);
    setQuery("");
  }

  function onKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Backspace" && query === "" && value.length > 0) {
      onChange(value.slice(0, -1));
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (needle && options.length > 0) add(options[0].id);
    }
  }

  return (
    <div className="flex flex-col gap-2">
      <label htmlFor={id} className="text-sm font-medium text-muted">
        {t("rooms.form.agents")}
      </label>
      <p className="-mt-1 text-xs text-muted">{hint ?? t("rooms.form.agentsHint")}</p>
      {selected.length > 0 && (
        <ul
          aria-label={t("rooms.form.selected")}
          className="flex flex-wrap gap-2"
        >
          {selected.map((agent) => (
            <li
              key={agent.id}
              className="flex max-w-full min-w-0 items-center gap-2 rounded-full border border-line bg-raised py-1 pr-1 pl-1.5 text-sm font-semibold text-ink"
            >
              <Avatar
                seed={agent.name}
                label={agent.name}
                className="size-6 shrink-0 rounded-full text-xs"
              />
              <span className="truncate">{agent.name}</span>
              <button
                type="button"
                aria-label={`${t("rooms.form.remove")} ${agent.name}`}
                onClick={() =>
                  onChange(value.filter((agentId) => agentId !== agent.id))
                }
                className="grid size-6 shrink-0 cursor-pointer place-items-center rounded-full text-muted hover:bg-hi hover:text-ink focus-visible:outline-2 focus-visible:outline-ink"
              >
                <X aria-hidden className="size-3.5" />
              </button>
            </li>
          ))}
        </ul>
      )}
      <div className="relative">
        <Search
          aria-hidden
          className="pointer-events-none absolute top-1/2 left-4 size-5 -translate-y-1/2 text-muted"
        />
        <input
          id={id}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={onKeyDown}
          placeholder={t("rooms.form.search")}
          className={`${fieldClasses} pl-12`}
        />
      </div>
      <div
        role="listbox"
        aria-label={t("rooms.form.available")}
        className="max-h-48 overflow-y-auto overscroll-contain rounded-2xl border border-line"
      >
        {options.map((agent) => (
          <div
            key={agent.id}
            role="option"
            aria-selected={false}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => add(agent.id)}
            className="flex cursor-pointer items-center gap-3 border-b border-line/60 px-3.5 py-2.5 text-sm last:border-b-0 hover:bg-hi/50"
          >
            <Avatar
              seed={agent.name}
              label={agent.name}
              className="size-7 shrink-0 rounded-full text-xs"
            />
            <span className="min-w-0 flex-1 truncate font-semibold text-ink">
              {agent.name}
            </span>
            <span className="max-w-[45%] truncate text-muted">
              {agent.role}
            </span>
          </div>
        ))}
        {options.length === 0 && (
          <p className="px-3.5 py-3 text-sm text-muted">
            {agents.length === 0
              ? t("rooms.form.noAgents")
              : t("rooms.form.noMatch")}
          </p>
        )}
      </div>
    </div>
  );
}
