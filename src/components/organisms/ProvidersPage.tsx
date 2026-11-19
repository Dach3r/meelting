import { useState } from "react";
import { useLlmProviders } from "../../hooks/useLlmProviders";
import { useLocalModels } from "../../hooks/useLocalModels";
import {
  useProviderKeys,
  type ProviderKeyStatus,
} from "../../hooks/useProviderKeys";
import { useI18n } from "../../i18n/I18nProvider";
import { translateError, translateWith } from "../../i18n/translate";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { LocalProviderRow } from "../molecules/LocalProviderRow";
import { Modal } from "../molecules/Modal";
import { ProviderKeyRow } from "../molecules/ProviderKeyRow";

export function ProvidersPage() {
  const { locale, t } = useI18n();
  const { statuses, checks, errorKey, save, check, remove } = useProviderKeys();
  const providers = useLlmProviders();
  const { detected, detectAgain } = useLocalModels(true);
  const [deleting, setDeleting] = useState<ProviderKeyStatus | null>(null);
  const local = providers.filter((provider) => provider.location === "local");
  const configured =
    statuses?.filter((status) => status.last_four !== null).length ?? 0;

  async function confirmDelete() {
    if (!deleting) return;
    const provider = deleting.provider;
    setDeleting(null);
    await remove(provider);
  }

  return (
    <div className="grid content-start gap-6">
      <div>
        <h1 className="text-2xl font-bold text-ink">{t("providers.title")}</h1>
        <p className="text-sm text-muted">{t("providers.description")}</p>
      </div>
      {errorKey && <Alert message={translateError(locale, errorKey)} />}
      <section className="grid gap-3">
        <div className="flex items-end justify-between gap-4">
          <div>
            <h2 className="text-base font-semibold text-ink">
              {t("providers.cloud.title")}
            </h2>
            <p className="text-sm text-muted">
              {t("providers.cloud.description")}
            </p>
          </div>
          {statuses && (
            <p className="shrink-0 text-sm font-medium text-muted">
              {translateWith(locale, "providers.cloud.summary", {
                count: configured,
                total: statuses.length,
              })}
            </p>
          )}
        </div>
        <div className="divide-y divide-line rounded-2xl bg-pill-hover">
          {statuses?.map((status) => (
            <ProviderKeyRow
              key={status.provider}
              status={status}
              check={checks[status.provider]}
              onSave={(key) => save(status.provider, key)}
              onCheck={() => check(status.provider)}
              onDelete={() => setDeleting(status)}
            />
          ))}
        </div>
      </section>
      <section className="grid gap-3">
        <div className="flex items-end justify-between gap-4">
          <div>
            <h2 className="text-base font-semibold text-ink">
              {t("providers.local.title")}
            </h2>
            <p className="text-sm text-muted">
              {t("providers.local.description")}
            </p>
          </div>
          <Button
            variant="ghost"
            size="compact"
            disabled={detected === null}
            onClick={detectAgain}
          >
            {t("providers.local.detectAgain")}
          </Button>
        </div>
        <div className="divide-y divide-line rounded-2xl bg-pill-hover">
          {local.map((provider) => {
            const found = detected?.find((entry) => entry.key === provider.key);
            return (
              <LocalProviderRow
                key={provider.key}
                name={provider.name}
                detecting={detected === null}
                models={found?.running ? found.models : null}
              />
            );
          })}
        </div>
      </section>
      {deleting && (
        <Modal
          title={translateWith(locale, "providers.deleteDialog.title", {
            provider: deleting.name,
          })}
          onClose={() => setDeleting(null)}
        >
          <div className="grid gap-5 p-6">
            <p className="text-muted">
              {translateWith(locale, "providers.deleteDialog.body", {
                provider: deleting.name,
              })}
            </p>
            <div className="flex justify-end gap-3">
              <Button
                type="button"
                variant="ghost"
                size="compact"
                onClick={() => setDeleting(null)}
              >
                {t("providers.actions.cancel")}
              </Button>
              <Button
                type="button"
                variant="destructive"
                size="compact"
                onClick={confirmDelete}
              >
                {t("providers.deleteDialog.confirm")}
              </Button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
