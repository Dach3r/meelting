import type { Participant } from "../../hooks/useRooms";

const queryMaxChars = 40;

export type ActiveMention = { start: number; query: string };

export type ChosenMention = { id: string; name: string };

function fold(text: string) {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

export function activeMention(
  body: string,
  caret: number,
): ActiveMention | null {
  const before = body.slice(0, caret);
  const start = before.lastIndexOf("@");
  if (start < 0 || /[\p{L}\p{N}]/u.test(before.charAt(start - 1))) return null;
  const query = before.slice(start + 1);
  if (query.length > queryMaxChars || /[\n@]/.test(query)) return null;
  return { start, query };
}

export function matchingAgents(agents: Participant[], query: string) {
  const wanted = fold(query);
  return agents.filter((agent) =>
    [agent.name, agent.role].some((label) => {
      const folded = fold(label);
      return (
        folded.startsWith(wanted) ||
        folded.split(/\s+/).some((word) => word.startsWith(wanted))
      );
    }),
  );
}

export function insertMention(
  body: string,
  mention: ActiveMention,
  caret: number,
  name: string,
) {
  const text = `@${name} `;
  return {
    body: body.slice(0, mention.start) + text + body.slice(caret),
    caret: mention.start + text.length,
  };
}

export function keptMentionIds(body: string, chosen: ChosenMention[]) {
  return [
    ...new Set(
      chosen
        .filter((mention) => body.includes(`@${mention.name}`))
        .map((mention) => mention.id),
    ),
  ];
}

export type DraftSegment = { text: string; agent: Participant | null };

export function draftSegments(
  body: string,
  agents: Participant[],
): DraftSegment[] {
  const names = agents
    .map((agent) => agent.name)
    .sort((a, b) => b.length - a.length)
    .map((name) => name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  if (names.length === 0) return [{ text: body, agent: null }];
  const pattern = new RegExp(
    `(?<![\\p{L}\\p{N}])@(${names.join("|")})(?![\\p{L}\\p{N}])`,
    "gu",
  );
  const segments: DraftSegment[] = [];
  let last = 0;
  for (const match of body.matchAll(pattern)) {
    if (match.index > last)
      segments.push({ text: body.slice(last, match.index), agent: null });
    segments.push({
      text: match[0],
      agent: agents.find((agent) => agent.name === match[1]) ?? null,
    });
    last = match.index + match[0].length;
  }
  if (last < body.length)
    segments.push({ text: body.slice(last), agent: null });
  return segments;
}
