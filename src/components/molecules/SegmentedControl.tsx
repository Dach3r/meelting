import type { LucideIcon } from "lucide-react";

export type SegmentedOption<T extends string> = {
  value: T;
  label: string;
  icon?: LucideIcon;
  lang?: string;
  pending?: boolean;
};

export function SegmentedControl<T extends string>({
  label,
  options,
  value,
  onChange,
  disabled = false,
  fullWidth = false,
}: {
  label: string;
  options: SegmentedOption<T>[];
  value: T;
  onChange: (value: T) => void;
  disabled?: boolean;
  fullWidth?: boolean;
}) {
  return (
    <nav
      aria-label={label}
      className={`flex items-center gap-1 rounded-full border border-line bg-pill p-1 text-sm ${fullWidth ? "w-full" : "w-fit"}`}
    >
      {options.map(({ icon: Icon, ...option }) => (
        <button
          key={option.value}
          type="button"
          lang={option.lang}
          disabled={disabled}
          aria-current={option.value === value ? "true" : undefined}
          onClick={() => onChange(option.value)}
          className={`inline-flex cursor-pointer items-center gap-2 rounded-full px-3 py-1 tracking-wider ${fullWidth ? "flex-1 justify-center" : ""} disabled:cursor-not-allowed ${option.value === value ? "bg-ink font-semibold text-surface" : "font-bold text-muted enabled:hover:text-ink"}`}
        >
          {Icon && <Icon aria-hidden className="size-4" />}
          {option.label}
          {option.pending && (
            <span className="ml-2 inline-block size-1.5 rounded-full bg-accent align-middle" />
          )}
        </button>
      ))}
    </nav>
  );
}
