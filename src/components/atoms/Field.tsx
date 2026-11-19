import type { LucideIcon } from "lucide-react";
import { useId, type InputHTMLAttributes, type ReactNode } from "react";

type Props = Omit<InputHTMLAttributes<HTMLInputElement>, "onChange"> & {
  label: string;
  value: string;
  onChange: (value: string) => void;
  multiline?: boolean;
  rows?: number;
  grow?: boolean;
  hint?: string;
  action?: ReactNode;
  icon?: LucideIcon;
};

export const fieldClasses =
  "w-full min-w-0 rounded-xl border border-line bg-raised px-4 py-4 text-base text-ink outline-none transition-colors placeholder:text-muted focus:bg-[color-mix(in_srgb,var(--color-raised)_75%,var(--color-surface))]";

export function Field({
  label,
  value,
  onChange,
  multiline = false,
  rows = 3,
  grow = false,
  hint,
  action,
  icon: Icon,
  ...rest
}: Props) {
  const id = useId();
  return (
    <div className={`flex flex-col gap-2 ${grow ? "flex-1" : ""}`}>
      <div className="flex items-center justify-between gap-3">
        <label htmlFor={id} className="text-sm font-medium text-muted">
          {label}
        </label>
        {action}
      </div>
      {hint && <p className="-mt-1 text-xs text-muted">{hint}</p>}
      {multiline ? (
        <textarea
          id={id}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          rows={rows}
          maxLength={rest.maxLength}
          placeholder={rest.placeholder}
          autoFocus={rest.autoFocus}
          className={`${fieldClasses} resize-none ${grow ? "min-h-32 flex-1" : "field-sizing-content min-h-20 max-h-40"}`}
        />
      ) : (
        <div className="relative">
          {Icon && (
            <Icon
              aria-hidden
              className="pointer-events-none absolute top-1/2 left-4 size-5 -translate-y-1/2 text-muted"
            />
          )}
          <input
            id={id}
            value={value}
            onChange={(event) => onChange(event.target.value)}
            className={`${fieldClasses} ${Icon ? "pl-12" : ""}`}
            {...rest}
          />
        </div>
      )}
    </div>
  );
}
