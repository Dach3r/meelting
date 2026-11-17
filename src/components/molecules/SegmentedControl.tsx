export type SegmentedOption<T extends string> = {
  value: T;
  label: string;
  lang?: string;
};

export function SegmentedControl<T extends string>({
  label,
  options,
  value,
  onChange,
  disabled = false,
}: {
  label: string;
  options: SegmentedOption<T>[];
  value: T;
  onChange: (value: T) => void;
  disabled?: boolean;
}) {
  return (
    <nav aria-label={label} className="flex w-fit items-center gap-1 rounded-full bg-shell p-1 text-sm">
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          lang={option.lang}
          disabled={disabled}
          aria-current={option.value === value ? "true" : undefined}
          onClick={() => onChange(option.value)}
          className={`cursor-pointer rounded-full px-3 py-1 tracking-wider disabled:cursor-not-allowed ${option.value === value ? "bg-ink font-semibold text-surface" : "font-bold text-muted enabled:hover:text-ink"}`}
        >
          {option.label}
        </button>
      ))}
    </nav>
  );
}
