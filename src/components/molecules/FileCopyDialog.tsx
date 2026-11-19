import { TriangleAlert } from "lucide-react";
import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { formatBytes, translateWith } from "../../i18n/translate";
import type { CopyPreview, CopyResult } from "../../hooks/useTasks";
import { Button } from "../atoms/Button";
import { Modal } from "./Modal";

type Props = {
  preview: CopyPreview;
  busy: boolean;
  onCopy: (acceptSecrets: boolean) => Promise<CopyResult[] | null>;
  onClose: () => void;
};

export function FileCopyDialog({ preview, busy, onCopy, onClose }: Props) {
  const { t, locale } = useI18n();
  const [accepted, setAccepted] = useState(false);
  const [results, setResults] = useState<CopyResult[] | null>(null);
  const copyable = preview.files.filter((file) => !file.problem);
  const secrets = copyable.some((file) => file.secret);

  async function copy() {
    const copied = await onCopy(accepted);
    if (copied) setResults(copied);
  }

  return (
    <Modal title={t("tasks.environment.copy.title")} onClose={onClose}>
      <div className="grid gap-4 p-6">
        {results ? (
          <ul className="grid max-h-72 overflow-y-auto rounded-2xl border border-line">
            {results.map((result) => (
              <li
                key={result.relative_path}
                className="flex min-w-0 items-center gap-3 border-b border-line/60 px-4 py-2 text-sm last:border-b-0"
              >
                <code className="min-w-0 flex-1 truncate">
                  {result.relative_path}
                </code>
                <span
                  className={`shrink-0 text-xs font-semibold ${result.outcome === "copied" ? "text-ink" : "text-error"}`}
                >
                  {t(`tasks.environment.copy.results.${result.outcome}`)}
                  {result.problem &&
                    ` · ${t(`tasks.environment.copy.problems.${result.problem}`)}`}
                </span>
              </li>
            ))}
          </ul>
        ) : (
          <>
            <p className="text-sm text-muted">
              {t("tasks.environment.copy.body")}
            </p>
            <ul
              aria-label={t("tasks.environment.copy.title")}
              className="grid max-h-72 overflow-y-auto rounded-2xl border border-line"
            >
              {preview.files.map((file, index) => (
                <li
                  key={`${index}:${file.relative_path}`}
                  className="grid gap-0.5 border-b border-line/60 px-4 py-2 text-sm last:border-b-0"
                >
                  <div className="flex min-w-0 items-center gap-3">
                    <code
                      className={`min-w-0 flex-1 truncate ${file.problem ? "text-muted line-through" : ""}`}
                    >
                      {file.relative_path || "—"}
                    </code>
                    {!file.problem && (
                      <span className="shrink-0 text-xs text-muted">
                        {formatBytes(locale, file.size_bytes)}
                      </span>
                    )}
                  </div>
                  {file.problem ? (
                    <span className="text-xs text-error">
                      {t(`tasks.environment.copy.problems.${file.problem}`)}
                    </span>
                  ) : (
                    file.secret && (
                      <span className="flex items-center gap-1 text-xs font-semibold text-error">
                        <TriangleAlert aria-hidden className="size-3.5" />
                        {t("tasks.environment.copy.secret")}
                      </span>
                    )
                  )}
                </li>
              ))}
            </ul>
            {copyable.length === 0 ? (
              <p className="text-sm text-muted">
                {t("tasks.environment.copy.none")}
              </p>
            ) : (
              <p className="text-sm text-ink">
                {translateWith(locale, "tasks.environment.copy.total", {
                  count: copyable.length,
                  size: formatBytes(locale, preview.total_bytes),
                })}
              </p>
            )}
            {secrets && (
              <div className="grid gap-3 rounded-2xl border border-error p-4">
                <p className="text-sm text-error">
                  {t("tasks.environment.copy.secretWarning")}
                </p>
                <label className="flex cursor-pointer items-center gap-2 text-sm font-medium text-ink">
                  <input
                    type="checkbox"
                    checked={accepted}
                    onChange={(event) => setAccepted(event.target.checked)}
                  />
                  {t("tasks.environment.copy.secretConfirm")}
                </label>
              </div>
            )}
          </>
        )}
        <div className="flex justify-end gap-3">
          {results ? (
            <Button type="button" size="compact" onClick={onClose}>
              {t("tasks.environment.copy.done")}
            </Button>
          ) : (
            <>
              <Button
                type="button"
                variant="ghost"
                size="compact"
                onClick={onClose}
              >
                {t("tasks.cancel")}
              </Button>
              <Button
                type="button"
                size="compact"
                disabled={
                  busy || copyable.length === 0 || (secrets && !accepted)
                }
                onClick={copy}
              >
                {t("tasks.environment.copy.submit")}
              </Button>
            </>
          )}
        </div>
      </div>
    </Modal>
  );
}
