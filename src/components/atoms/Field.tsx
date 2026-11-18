import type { LucideIcon } from "lucide-react";
import { useId, type InputHTMLAttributes } from "react";

type Props = Omit<InputHTMLAttributes<HTMLInputElement>, "onChange"> & {
  label: string;
  value: string;
  onChange: (value: string) => void;
  multiline?: boolean;
  icon?: LucideIcon;
};

export const fieldClasses =
  "w-full min-w-0 rounded-xl bg-raised px-4 py-4 text-base text-ink outline-none transition-colors placeholder:text-muted focus:bg-hi";

export function Field({ label, value, onChange, multiline = false, icon: Icon, ...rest }: Props) {
  const id = useId();
  return (
    <div className="grid gap-2">
      <label htmlFor={id} className="text-sm font-medium text-muted">
        {label}
      </label>
      {multiline ? (
        <textarea
          id={id}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          rows={4}
          maxLength={rest.maxLength}
          className={fieldClasses}
        />
      ) : (
        <div className="relative">
          {Icon && <Icon aria-hidden className="pointer-events-none absolute top-1/2 left-4 size-5 -translate-y-1/2 text-muted" />}
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
