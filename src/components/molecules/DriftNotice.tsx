import { Info } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import type { DriftState } from "../../hooks/useEnvironmentReview";

export function DriftNotice({ state }: { state: DriftState }) {
  const { t } = useI18n();
  if (state === "up_to_date") return null;
  return (
    <p
      role="status"
      data-drift={state}
      className="flex items-start gap-3 rounded-2xl border border-line bg-raised px-4 py-3 text-sm text-ink"
    >
      <Info aria-hidden className="mt-0.5 size-4 shrink-0 text-muted" />
      {t(`review.drift.${state}`)}
    </p>
  );
}
