import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { I18nProvider } from "../../i18n/I18nProvider";
import { diff, entry } from "../../test/review";
import { DiffViewer, hunkStarts } from "./DiffViewer";

describe("hunkStarts", () => {
  it("reads the first old and new line of a hunk header", () => {
    expect(hunkStarts("@@ -3,2 +10,3 @@ fn main()")).toEqual({
      oldLine: 3,
      newLine: 10,
    });
    expect(hunkStarts("@@ -1 +1 @@")).toEqual({ oldLine: 1, newLine: 1 });
    expect(hunkStarts("not a header")).toBeNull();
  });
});

describe("DiffViewer", () => {
  type State = Parameters<typeof DiffViewer>[0]["state"];

  function renderViewer(locale: "en" | "es", state: State, path = "a.ts") {
    return render(
      <I18nProvider locale={locale}>
        <DiffViewer
          row={{ entry: entry(0, path), source: "staged" }}
          state={state}
        />
      </I18nProvider>,
    );
  }

  it("labels the no-newline marker and never marks it as added or removed", () => {
    renderViewer("en", {
      phase: "ready",
      diff: diff({
        hunks: [
          {
            header: "@@ -1 +1 @@",
            lines: [
              { kind: "added", text: "x" },
              { kind: "no_newline", text: " No newline at end of file" },
            ],
          },
        ],
      }),
    });

    expect(screen.getByText("No newline at end of file")).toBeVisible();
    expect(screen.getAllByText("Added line")).toHaveLength(1);
  });

  it("speaks Spanish", () => {
    renderViewer("es", {
      phase: "ready",
      diff: diff({ binary: true, hunks: [] }),
    });

    expect(screen.getByText(/archivo binario/)).toBeVisible();
    expect(screen.getByText("cambios en staging")).toBeVisible();
  });

  it("renders hostile file names and contents as plain text", () => {
    renderViewer("en", { phase: "ready", diff: diff() }, "<b>x</b>.ts");

    expect(screen.getAllByText("<b>x</b>.ts").length).toBeGreaterThan(0);
    expect(document.querySelector("b")).toBeNull();
    expect(document.querySelector("img")).toBeNull();
  });
});
