import { useEffect, type ReactNode } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { CloseIcon } from "../atoms/Icons";

type Props = {
  title: string;
  onClose: () => void;
  bare?: boolean;
  children: ReactNode;
};

export function CloseButton({ onClick, label }: { onClick: () => void; label: string }) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className="grid size-10 shrink-0 cursor-pointer place-items-center rounded-full bg-raised text-muted transition-colors hover:bg-hi hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
    >
      <CloseIcon />
    </button>
  );
}

export function Modal({ title, onClose, bare = false, children }: Props) {
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
        className={`max-h-full w-full overflow-y-auto rounded-[28px] border border-line bg-surface shadow-2xl ${bare ? "max-w-3xl" : "max-w-2xl"}`}
      >
        {!bare && (
          <div className="flex items-center justify-between px-6 pt-6">
            <h2 className="pl-1 text-xl font-bold text-ink">{title}</h2>
            <CloseButton onClick={onClose} label={t("teams.close")} />
          </div>
        )}
        {children}
      </div>
    </div>
  );
}
