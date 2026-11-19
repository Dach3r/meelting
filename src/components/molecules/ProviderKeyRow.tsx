import { useState, type FormEvent } from "react";
import type {
  KeyAttempt,
  ProviderKeyStatus,
  Verification,
} from "../../hooks/useProviderKeys";
import { useI18n } from "../../i18n/I18nProvider";
import { translateError, translateWith } from "../../i18n/translate";
import { Button } from "../atoms/Button";
import { Field } from "../atoms/Field";

type Props = {
  status: ProviderKeyStatus;
  check: Verification | undefined;
  onSave: (key: string) => Promise<KeyAttempt>;
  onCheck: () => Promise<KeyAttempt>;
  onDelete: () => void;
};

type Tone = "ok" | "neutral" | "problem";

const dotTones: Record<Tone, string> = {
  ok: "bg-success",
  neutral: "bg-muted",
  problem: "bg-error",
};

export function ProviderKeyRow({
  status,
  check,
  onSave,
  onCheck,
  onDelete,
}: Props) {
  const { locale, t } = useI18n();
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [pending, setPending] = useState<"save" | "check" | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const values = { provider: status.name, lastFour: status.last_four ?? "" };

  function describe(verification: Verification) {
    if (verification.status === "rejected") {
      const key = `providers.rejected.${verification.reason}`;
      const known = translateWith(locale, key, values) !== key;
      return translateWith(
        locale,
        known ? key : "providers.rejected.default",
        values,
      );
    }
    return translateWith(
      locale,
      `providers.outcome.${verification.status}`,
      values,
    );
  }

  function presentation(): [Tone, string] {
    if (status.last_four === null)
      return ["neutral", t("providers.status.notConfigured")];
    if (check?.status === "verified")
      return [
        "ok",
        translateWith(locale, "providers.status.connected", values),
      ];
    if (check?.status === "rejected")
      return ["problem", t("providers.status.rejected")];
    if (check?.status === "offline" || status.offline)
      return ["problem", t("providers.status.offline")];
    if (check?.status === "provider_busy")
      return [
        "problem",
        translateWith(locale, "providers.status.busy", values),
      ];
    return [
      "neutral",
      translateWith(locale, "providers.status.configured", values),
    ];
  }

  function close() {
    setEditing(false);
    setDraft("");
    setMessage(null);
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    const key = draft;
    setDraft("");
    setPending("save");
    setMessage(null);
    const result = await onSave(key);
    setPending(null);
    if ("error" in result) {
      setMessage(translateError(locale, result.error));
    } else if (result.verification.status === "verified") {
      close();
    } else {
      const kept = result.last_four
        ? ` ${translateWith(locale, "providers.keptPrevious", { lastFour: result.last_four })}`
        : "";
      setMessage(`${describe(result.verification)}${kept}`);
    }
  }

  async function runCheck() {
    setPending("check");
    setMessage(null);
    const result = await onCheck();
    setPending(null);
    if ("error" in result) setMessage(translateError(locale, result.error));
  }

  const configured = status.last_four !== null;
  const [tone, label] = presentation();
  const checkDetail =
    !editing && check && check.status !== "verified" ? describe(check) : null;

  return (
    <div className="flex flex-col gap-4 rounded-2xl bg-pill-hover px-5 py-4">
      <div className="flex flex-wrap items-center gap-x-6 gap-y-3">
        <div className="min-w-0 flex-1">
          <h3 className="text-base font-semibold text-ink">{status.name}</h3>
          <p className="flex items-center gap-2 text-sm text-muted">
            <span
              aria-hidden
              className={`size-2 shrink-0 rounded-full ${dotTones[tone]}`}
            />
            {label}
          </p>
        </div>
        {!editing && (
          <div className="flex flex-wrap items-center gap-2">
            {configured ? (
              <>
                <Button
                  variant="ghost"
                  size="compact"
                  disabled={pending !== null}
                  onClick={runCheck}
                >
                  {pending === "check"
                    ? t("providers.actions.checking")
                    : t("providers.actions.check")}
                </Button>
                <Button
                  variant="ghost"
                  size="compact"
                  disabled={pending !== null}
                  onClick={() => setEditing(true)}
                >
                  {t("providers.actions.replace")}
                </Button>
                <Button
                  variant="danger"
                  size="compact"
                  disabled={pending !== null}
                  onClick={onDelete}
                >
                  {t("providers.actions.delete")}
                </Button>
              </>
            ) : (
              <Button size="compact" onClick={() => setEditing(true)}>
                {t("providers.actions.add")}
              </Button>
            )}
          </div>
        )}
      </div>
      {checkDetail && <p className="text-sm text-error">{checkDetail}</p>}
      {editing && (
        <form onSubmit={submit} className="flex flex-col gap-3">
          <Field
            label={translateWith(locale, "providers.keyLabel", values)}
            value={draft}
            onChange={setDraft}
            type="password"
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
            maxLength={512}
            disabled={pending !== null}
            autoFocus
          />
          {message && <p className="text-sm text-error">{message}</p>}
          <div className="flex justify-end gap-2">
            <Button
              type="button"
              variant="ghost"
              size="compact"
              disabled={pending !== null}
              onClick={close}
            >
              {t("providers.actions.cancel")}
            </Button>
            <Button
              type="submit"
              size="compact"
              disabled={pending !== null || draft.trim() === ""}
            >
              {pending === "save"
                ? t("providers.actions.verifying")
                : t("providers.actions.save")}
            </Button>
          </div>
        </form>
      )}
      {!editing && message && <p className="text-sm text-error">{message}</p>}
    </div>
  );
}
