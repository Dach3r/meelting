import { useId } from "react";
import { fieldClasses } from "./Field";

type Props = {
  label: string;
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
  hint?: string;
  disabled?: boolean;
};

export function Select({
  label,
  value,
  options,
  onChange,
  hint,
  disabled,
}: Props) {
  const id = useId();
  return (
    <div className="flex min-w-0 flex-col gap-2">
      <label htmlFor={id} className="text-sm font-medium text-muted">
        {label}
      </label>
      {hint && <p className="-mt-1 text-xs text-muted">{hint}</p>}
      <select
        id={id}
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
        className={`${fieldClasses} cursor-pointer py-3 disabled:cursor-not-allowed disabled:opacity-60`}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </div>
  );
}
