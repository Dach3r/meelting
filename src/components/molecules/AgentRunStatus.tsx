import { CircleStop, LoaderCircle, TriangleAlert } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { translateRunError, translateWith } from "../../i18n/translate";
import { type AgentRun, isActive } from "../../hooks/agentRunState";
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

function statusText(locale: "es" | "en", run: AgentRun, agentName: string, providerName: string, local: boolean) {
  const agent = { agent: agentName };
  if (run.status === "failed") return translateRunError(locale, run.error_kind, providerName, local);
  if (run.status === "succeeded") return translateWith(locale, "agentRun.succeeded", agent);
  if (run.status === "running") return translateWith(locale, "agentRun.running", agent);
  if (run.attempts > 0) return translateWith(locale, "agentRun.retrying", { ...agent, attempt: run.attempts + 1 });
  return translateWith(locale, "agentRun.queued", agent);
}

export function AgentRunStatus({ run, agentName, providerName, local, liveText, onCancel }: Props) {
  const { t, locale } = useI18n();
  const active = isActive(run);
  const failed = run.status === "failed" && run.error_kind !== "cancelled";
  return (
    <li className="grid gap-3 pl-11">
      {liveText && (
        <div className="flex min-w-0 gap-3" aria-live="polite">
          <Avatar seed={agentName} label={agentName} className="size-8 shrink-0 rounded-full text-sm" />
          <div className="grid min-w-0 flex-1 gap-1">
            <p className="truncate text-sm font-semibold text-ink">{agentName}</p>
            <p className="text-sm whitespace-pre-wrap break-words text-ink/80 select-text">{liveText}</p>
          </div>
        </div>
      )}
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
        <p role="status" className={`flex items-center gap-1.5 font-medium ${failed ? "text-error" : "text-muted"}`}>
          {active && <LoaderCircle aria-hidden className="size-3.5 animate-spin" />}
          {failed && <TriangleAlert aria-hidden className="size-3.5" />}
          {statusText(locale, run, agentName, providerName, local)}
        </p>
        <span className="text-muted">
          {providerName} · {run.model}
        </span>
        {active && (
          <Button type="button" variant="ghost" size="compact" onClick={onCancel}>
            <CircleStop aria-hidden className="size-4" />
            {t("agentRun.cancel")}
          </Button>
        )}
      </div>
    </li>
  );
}
