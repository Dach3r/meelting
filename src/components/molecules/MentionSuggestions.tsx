import type { Participant } from "../../hooks/useRooms";
import { Avatar } from "../atoms/Avatar";
import { roleRepeatsName } from "./agentLabel";

type Props = {
  id: string;
  label: string;
  agents: Participant[];
  active: number;
  onPick: (agent: Participant) => void;
};

export function mentionOptionId(listId: string, index: number) {
  return `${listId}-${index}`;
}

export function MentionSuggestions({ id, label, agents, active, onPick }: Props) {
  return (
    <ul
      id={id}
      role="listbox"
      aria-label={label}
      className="absolute inset-x-0 bottom-full z-10 mb-2 grid max-h-60 gap-0.5 overflow-y-auto rounded-xl border border-line bg-surface p-1 shadow-lg"
    >
      {agents.map((agent, index) => (
        <li
          key={agent.id}
          id={mentionOptionId(id, index)}
          role="option"
          aria-selected={index === active}
          onMouseDown={(event) => {
            event.preventDefault();
            onPick(agent);
          }}
          className={`flex cursor-pointer items-center gap-2 rounded-lg px-2 py-1.5 text-sm ${index === active ? "bg-ink/8" : "hover:bg-ink/5"}`}
        >
          <Avatar seed={agent.name} label={agent.name} className="size-6 shrink-0 rounded-full text-xs" />
          <span className="truncate font-medium text-ink">{agent.name}</span>
          {!roleRepeatsName(agent.name, agent.role) && <span className="truncate text-xs text-muted">{agent.role}</span>}
        </li>
      ))}
    </ul>
  );
}
