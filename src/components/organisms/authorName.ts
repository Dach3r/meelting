import type { Message, Room } from "../../hooks/useRooms";

export function authorName(t: (key: string) => string, room: Room, message: Message) {
  if (message.agent_id === null) return t("rooms.you");
  return room.agents.find((agent) => agent.id === message.agent_id)?.name ?? "";
}
