import { useEffect, type ReactNode } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { CloseIcon } from "../atoms/Icons";

type Props = {
  icon: ReactNode;
  title: string;
  subtitle: string;
  onClose: () => void;
  children: ReactNode;
};

export function Modal({ icon, title, subtitle, onClose, children }: Props) {
  const { t } = useI18n();
  useEffect(() => {
    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    document.addEventListener("keydown", closeOnEscape);
    return () => document.removeEventListener("keydown", closeOnEscape);
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/60 p-4"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className="max-h-full w-full max-w-2xl overflow-y-auto rounded-2xl border border-line bg-raised"
      >
        <div className="flex items-center gap-4 px-6 pt-5">
          <span className="grid size-11 shrink-0 place-items-center rounded-xl bg-primary text-accent">{icon}</span>
          <div className="min-w-0 flex-1">
            <h2 className="text-lg font-bold text-ink">{title}</h2>
            <p className="truncate text-sm text-muted">{subtitle}</p>
          </div>
          <button
            type="button"
            aria-label={t("teams.close")}
            onClick={onClose}
            className="grid size-9 cursor-pointer place-items-center rounded-xl text-muted transition-colors hover:bg-surface hover:text-ink focus-visible:outline-2 focus-visible:outline-ink"
          >
            <CloseIcon />
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
