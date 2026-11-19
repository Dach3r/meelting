import type { Message, Participant } from "../../hooks/useRooms";

export function authorName(
  t: (key: string) => string,
  agents: Participant[],
  message: Message,
) {
  if (message.agent_id === null) return t("rooms.you");
  return agents.find((agent) => agent.id === message.agent_id)?.name ?? "";
}
