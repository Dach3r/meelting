export function Switch({
  label,
  checked,
  onChange,
  disabled = false,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`relative inline-flex h-7 w-12 shrink-0 cursor-pointer items-center rounded-full border border-line transition-colors duration-200 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink disabled:cursor-not-allowed disabled:opacity-60 motion-reduce:transition-none ${checked ? "bg-ink" : "bg-pill"}`}
    >
      <span
        aria-hidden
        className={`absolute left-0.5 size-5 rounded-full transition-transform duration-200 motion-reduce:transition-none ${checked ? "translate-x-5 bg-surface" : "translate-x-0 bg-muted"}`}
      />
    </button>
  );
}
