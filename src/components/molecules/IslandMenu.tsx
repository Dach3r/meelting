import { useEffect, useId, useRef, useState, type ReactNode } from "react";

type Props = {
  label: string;
  trigger: ReactNode;
  children: (close: () => void) => ReactNode;
};

export function IslandMenu({ label, trigger, children }: Props) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const filterId = `island-goo-${useId().replace(/:/g, "")}`;

  useEffect(() => {
    if (!open) return;
    function closeOutside(event: MouseEvent) {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    }
    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") setOpen(false);
    }
    document.addEventListener("mousedown", closeOutside);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("mousedown", closeOutside);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [open]);

  return (
    <div ref={root} data-open={open || undefined} className="group/island relative z-20 w-fit">
      <svg className="absolute size-0" aria-hidden="true" focusable="false">
        <filter id={filterId} colorInterpolationFilters="sRGB">
          <feGaussianBlur in="SourceGraphic" stdDeviation="9" result="blur" />
          <feColorMatrix in="blur" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 20 -9" result="shape" />
          <feFlood className="[flood-color:var(--color-surface)] group-hover/island:[flood-color:var(--color-raised)] group-data-open/island:[flood-color:var(--color-raised)]" result="paint" />
          <feComposite in="paint" in2="shape" operator="in" />
        </filter>
      </svg>
      <div
        className="pointer-events-none absolute -bottom-3 top-17 left-0 w-[calc(100%+0.75rem)] rounded-br-4xl bg-shell opacity-0 transition-opacity duration-150 before:absolute before:top-full before:left-0 before:size-6 before:bg-[radial-gradient(circle_at_100%_100%,transparent_23.5px,var(--color-shell)_24px)] after:absolute after:top-0 after:left-full after:size-6 after:bg-[radial-gradient(circle_at_100%_100%,transparent_23.5px,var(--color-shell)_24px)] group-data-open/island:opacity-100 group-data-open/island:duration-300 sm:top-18"
        aria-hidden="true"
      />
      <div className="pointer-events-none absolute inset-0" style={{ filter: `url(#${filterId})` }} aria-hidden="true">
        <div className="absolute inset-x-0 top-0 h-14 rounded-full bg-surface transition-colors group-hover/island:bg-raised group-data-open/island:rounded-t-[1.75rem] group-data-open/island:rounded-b-none group-data-open/island:bg-raised sm:h-15 sm:group-data-open/island:rounded-t-[1.875rem]" />
        <div className="absolute inset-x-0 top-8 bottom-0 rounded-4xl bg-raised opacity-0 transition-opacity duration-300 group-data-open/island:opacity-100" />
      </div>
      <button
        type="button"
        aria-haspopup="true"
        aria-expanded={open}
        aria-label={label}
        onClick={() => setOpen(!open)}
        className="relative flex w-full cursor-pointer items-center gap-3 rounded-full py-2 pr-4 pl-2 text-left outline-none sm:gap-4 sm:py-2.5 sm:pr-6 sm:pl-2.5"
      >
        {trigger}
      </button>
      <div className="relative grid grid-rows-[0fr] text-ink transition-[grid-template-rows] duration-500 ease-out group-data-open/island:grid-rows-[1fr] group-data-open/island:ease-[cubic-bezier(0.34,1.3,0.64,1)]">
        <div className="invisible min-h-0 min-w-72 overflow-hidden px-2 opacity-0 transition-[opacity,visibility] duration-200 group-data-open/island:visible group-data-open/island:opacity-100 group-data-open/island:delay-150 group-data-open/island:duration-300">
          <div className="pt-1 pb-3">{children(() => setOpen(false))}</div>
        </div>
      </div>
    </div>
  );
}
