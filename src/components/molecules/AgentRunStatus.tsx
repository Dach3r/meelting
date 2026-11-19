import { CircleStop, LoaderCircle, TriangleAlert } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { translateRunError, translateWith } from "../../i18n/translate";
import type { AgentRun } from "../../hooks/agentRunState";
import { Avatar } from "../atoms/Avatar";
import { Button } from "../atoms/Button";

type Props = {
  run: AgentRun;
  agentName: string;
  providerName: string;
  local: boolean;
  liveText: string | null;
  onCancel: () => void;
};

export function AgentRunStatus({
  run,
  agentName,
  providerName,
  local,
  liveText,
  onCancel,
}: Props) {
  const { t, locale } = useI18n();
  if (run.status === "succeeded") return null;
  if (run.status === "failed") {
    const cancelled = run.error_kind === "cancelled";
    return (
      <li className="pl-11">
        <p
          role="status"
          className={`flex items-center gap-1.5 text-xs font-medium ${cancelled ? "text-muted" : "text-error"}`}
        >
          {!cancelled && (
            <TriangleAlert aria-hidden className="size-3.5 shrink-0" />
          )}
          {translateRunError(locale, run.error_kind, providerName, local)}
        </p>
      </li>
    );
  }
  const status =
    run.status === "running"
      ? t(liveText ? "agentRun.writing" : "agentRun.thinking")
      : run.attempts > 0
        ? translateWith(locale, "agentRun.retrying", {
            attempt: run.attempts + 1,
          })
        : t("agentRun.queued");
  return (
    <li className="flex min-w-0 gap-3">
      <Avatar
        seed={agentName}
        label={agentName}
        className="size-8 shrink-0 rounded-full text-sm"
      />
      <div className="grid min-w-0 flex-1 gap-1">
        <div className="flex min-w-0 flex-wrap items-center gap-x-2 text-sm">
          <span className="truncate font-semibold text-ink">{agentName}</span>
          <span
            role="status"
            className="flex items-center gap-1.5 text-xs text-muted"
          >
            <LoaderCircle aria-hidden className="size-3.5 animate-spin" />
            {status}
          </span>
          <Button
            type="button"
            variant="ghost"
            size="compact"
            onClick={onCancel}
          >
            <CircleStop aria-hidden className="size-4" />
            {t("agentRun.cancel")}
          </Button>
        </div>
        {liveText && (
          <p
            aria-live="polite"
            className="text-sm whitespace-pre-wrap break-words text-ink/80 select-text"
          >
            {liveText}
          </p>
        )}
      </div>
    </li>
  );
}
