import { useI18n } from "../../i18n/I18nProvider";

const barHeights = ["h-[36%]", "h-[66%]", "h-full", "h-[66%]", "h-[36%]"];

export function SwitchingOverlay({ workspaceName }: { workspaceName: string }) {
  const { t } = useI18n();

  return (
    <div
      role="status"
      className="absolute inset-0 z-50 grid animate-fade-in place-content-center justify-items-center gap-3 bg-rail/70 backdrop-blur-md motion-reduce:animate-none"
    >
      <div className="flex h-9 items-center gap-1" aria-hidden="true">
        {barHeights.map((height, index) => (
          <span
            key={index}
            style={{ animationDelay: `${index * 0.12}s` }}
            className={`w-1 animate-equalizer rounded-full bg-ink motion-reduce:animate-none ${height}`}
          />
        ))}
      </div>
      <p className="text-sm text-muted">
        {t("home.workspace.switching")} <b className="font-semibold text-ink">{workspaceName}</b>
      </p>
    </div>
  );
}
