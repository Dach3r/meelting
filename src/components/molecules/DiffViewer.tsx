import { useI18n } from "../../i18n/I18nProvider";
import { translateError, translateWith } from "../../i18n/translate";
import type {
  DiffLineKind,
  DiffState,
  FileDiff,
  Row,
} from "../../hooks/useEnvironmentReview";

const marks: Record<DiffLineKind, string> = {
  context: " ",
  added: "+",
  removed: "−",
  no_newline: "",
};

const tones: Record<DiffLineKind, string> = {
  context: "",
  added: "bg-success/10",
  removed: "bg-error/10",
  no_newline: "text-muted",
};

export function hunkStarts(header: string) {
  const match = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(header);
  return match ? { oldLine: Number(match[1]), newLine: Number(match[2]) } : null;
}

type Props = { row: Row | null; state: DiffState | null };

export function DiffViewer({ row, state }: Props) {
  const { t, locale } = useI18n();
  if (!row || !state) {
    return <p className="text-sm text-muted">{t("review.diff.select")}</p>;
  }
  const title = translateWith(locale, "review.diff.region", {
    path: row.entry.path,
    source: t(`review.diff.sources.${row.source ?? "branch"}`),
  });
  return (
    <section aria-label={title} className="grid min-w-0 gap-3">
      <h3 className="min-w-0 text-sm font-semibold break-all text-ink">
        <code>{row.entry.path}</code>
        <span className="ml-2 font-normal text-muted">
          {t(`review.diff.sources.${row.source ?? "branch"}`)}
        </span>
      </h3>
      {state.phase === "loading" && (
        <p role="status" className="text-sm text-muted">
          {t("review.diff.loading")}
        </p>
      )}
      {state.phase === "protected" && (
        <p role="note" className="text-sm text-muted">
          {t("review.diff.protected")}
        </p>
      )}
      {state.phase === "error" && (
        <p role="alert" className="text-sm font-medium text-error">
          {translateError(locale, state.errorKey)}
        </p>
      )}
      {state.phase === "ready" && <DiffBody diff={state.diff} title={title} />}
    </section>
  );
}

function DiffBody({ diff, title }: { diff: FileDiff; title: string }) {
  const { t } = useI18n();
  return (
    <>
      {diff.lossy && (
        <p className="text-xs text-muted">{t("review.diff.lossy")}</p>
      )}
      {diff.binary && (
        <p role="note" className="text-sm text-muted">
          {t("review.diff.binary")}
        </p>
      )}
      {!diff.binary && diff.hunks.length === 0 && (
        <p role="note" className="text-sm text-muted">
          {t("review.diff.noChanges")}
        </p>
      )}
      {diff.hunks.length > 0 && (
        <div className="overflow-x-auto rounded-2xl border border-line bg-surface">
          <table aria-label={title} className="w-full border-collapse font-mono text-xs select-text">
            {diff.hunks.map((hunk, index) => {
              const starts = hunkStarts(hunk.header);
              let oldLine = starts?.oldLine ?? 0;
              let newLine = starts?.newLine ?? 0;
              return (
                <tbody key={index}>
                  <tr className="bg-raised text-muted">
                    <th
                      colSpan={4}
                      scope="colgroup"
                      className="px-3 py-1 text-left font-normal"
                    >
                      {hunk.header}
                    </th>
                  </tr>
                  {hunk.lines.map((line, lineIndex) => {
                    const old = line.kind === "context" || line.kind === "removed";
                    const next = line.kind === "context" || line.kind === "added";
                    const oldNumber = old && starts ? oldLine++ : null;
                    const newNumber = next && starts ? newLine++ : null;
                    return (
                      <tr key={lineIndex} className={tones[line.kind]}>
                        <td
                          aria-hidden
                          className="w-12 min-w-12 px-2 text-right text-muted select-none"
                        >
                          {oldNumber}
                        </td>
                        <td
                          aria-hidden
                          className="w-12 min-w-12 px-2 text-right text-muted select-none"
                        >
                          {newNumber}
                        </td>
                        <td className="w-6 px-1 text-center font-bold select-none">
                          <span aria-hidden>{marks[line.kind]}</span>
                          <span className="sr-only">
                            {line.kind === "no_newline"
                              ? ""
                              : t(`review.diff.lines.${line.kind}`)}
                          </span>
                        </td>
                        <td className="px-2 whitespace-pre">
                          {line.kind === "no_newline" ? (
                            <em>{t("review.diff.noNewline")}</em>
                          ) : (
                            line.text
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              );
            })}
          </table>
        </div>
      )}
      {!diff.complete && (
        <p role="note" className="text-xs text-muted">
          {t("review.diff.partial")}
        </p>
      )}
    </>
  );
}
