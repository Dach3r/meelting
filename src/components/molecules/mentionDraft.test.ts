import { describe, expect, it } from "vitest";
import type { Participant } from "../../hooks/useRooms";
import { draftSegments } from "./mentionDraft";

function agent(id: string, name: string): Participant {
  return { id, name, role: "Developer", provider: "ollama", model: "llama3" };
}

const agents = [agent("a1", "Ana"), agent("a2", "Backend Developer")];

describe("draftSegments", () => {
  it("marks the mentions of agents in the room, including names with spaces", () => {
    const segments = draftSegments("@Backend Developer como estas? @Ana", agents);

    expect(segments.map((s) => [s.text, s.agent?.id ?? null])).toEqual([
      ["@Backend Developer", "a2"],
      [" como estas? ", null],
      ["@Ana", "a1"],
    ]);
  });

  it("ignores unknown names, partial names and mentions inside words", () => {
    const segments = draftSegments("@Anabel mail@Ana @Bob", agents);

    expect(segments).toEqual([
      { text: "@Anabel mail@Ana @Bob", agent: null },
    ]);
  });

  it("returns the text as is without agents", () => {
    expect(draftSegments("hi", [])).toEqual([{ text: "hi", agent: null }]);
  });
});
