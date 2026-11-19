import { useI18n } from "../../i18n/I18nProvider";
import { Button } from "../atoms/Button";
import { Alert } from "./Alert";

const barHeights = ["h-[36%]", "h-[66%]", "h-full", "h-[66%]", "h-[36%]"];

export const transitionMs = 600;

type Props = {
  label: string;
  name: string;
  errorMessage?: string | null;
  onDismiss?: () => void;
};

export function TransitionOverlay({ label, name, errorMessage = null, onDismiss }: Props) {
  const { t } = useI18n();

  return (
    <div className="absolute inset-0 z-50 grid animate-fade-in place-content-center justify-items-center gap-3 bg-shell/70 backdrop-blur-md motion-reduce:animate-none">
      {errorMessage ? (
        <>
          <Alert message={errorMessage} />
          <Button variant="ghost" size="compact" onClick={onDismiss}>
            {t("home.workspace.dismiss")}
          </Button>
        </>
      ) : (
        <div role="status" className="grid justify-items-center gap-3">
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
            {label} <b className="font-semibold text-ink">{name}</b>
          </p>
        </div>
      )}
    </div>
  );
}
