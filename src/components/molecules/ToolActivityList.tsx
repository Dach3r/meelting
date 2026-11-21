import { Check, FileDiff, LoaderCircle, X } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { type Locale, translateWith } from "../../i18n/translate";
import type { ToolActivity } from "../../hooks/useRoom";
import { Button } from "../atoms/Button";

const knownTools = [
  "list_dir",
  "read_file",
  "search",
  "git_status",
  "git_diff",
  "write_file",
  "edit_file",
];

export function toolLabel(
  locale: Locale,
  activity: ToolActivity,
  live: boolean,
) {
  const tool = activity.tool ?? "unknown";
  const key = knownTools.includes(tool) ? tool : "unknown";
  return translateWith(locale, `tools.${live ? "live" : "done"}.${key}`, {
    path: activity.path ?? ".",
    tool,
  });
}

export function changedFiles(calls: ToolActivity[]) {
  return calls.some(
    (call) =>
      call.status === "succeeded" &&
      (call.tool === "write_file" || call.tool === "edit_file"),
  );
}

export function ToolActivityList({
  calls,
  onOpenChanges,
}: {
  calls: ToolActivity[];
  onOpenChanges: (taskId: string) => void;
}) {
  const { t, locale } = useI18n();
  if (calls.length === 0) return null;
  return (
    <li className="grid gap-2 rounded-xl border border-line bg-shell px-3 py-2.5">
      <p className="text-xs font-semibold tracking-wide text-muted uppercase">
        {t("tools.title")}
      </p>
      <ul aria-label={t("tools.title")} className="grid gap-1.5">
        {calls.map((call) => (
          <li key={call.id} className="flex items-start gap-2 text-sm">
            <StatusIcon status={call.status} />
            <span className="min-w-0 flex-1 break-words text-ink">
              {toolLabel(locale, call, call.status === "running")}
              {call.status === "failed" && (
                <span className="text-error">
                  {" · "}
                  {t("tools.failed")}
                </span>
              )}
            </span>
          </li>
        ))}
      </ul>
      {changedFiles(calls) && (
        <Button
          type="button"
          variant="ghost"
          size="compact"
          className="justify-self-start"
          onClick={() => onOpenChanges(calls[0].task_id)}
        >
          <FileDiff aria-hidden className="size-4" />
          {t("tools.viewChanges")}
        </Button>
      )}
    </li>
  );
}

function StatusIcon({ status }: { status: ToolActivity["status"] }) {
  const { t } = useI18n();
  const label = t(`tools.status.${status}`);
  if (status === "running")
    return (
      <LoaderCircle
        aria-label={label}
        className="mt-0.5 size-4 shrink-0 animate-spin text-accent motion-reduce:animate-none"
      />
    );
  if (status === "succeeded")
    return (
      <Check
        aria-label={label}
        className="mt-0.5 size-4 shrink-0 text-accent"
      />
    );
  return <X aria-label={label} className="mt-0.5 size-4 shrink-0 text-error" />;
}
