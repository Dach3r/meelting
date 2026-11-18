import { ChevronDown, type LucideIcon } from "lucide-react";
import { useId } from "react";
import { fieldClasses } from "./Field";

type Props = {
  label: string;
  placeholder: string;
  value: string;
  options: string[];
  onChange: (value: string) => void;
  icon?: LucideIcon;
  autoFocus?: boolean;
};

export function SelectField({ label, placeholder, value, options, onChange, icon: Icon, autoFocus }: Props) {
  const id = useId();
  return (
    <div className="grid gap-2">
      <label htmlFor={id} className="text-sm font-medium text-muted">
        {label}
      </label>
      <div className="relative">
        {Icon && <Icon aria-hidden className="pointer-events-none absolute top-1/2 left-4 size-5 -translate-y-1/2 text-muted" />}
        <select
          id={id}
          value={value}
          autoFocus={autoFocus}
          onChange={(event) => onChange(event.target.value)}
          className={`${fieldClasses} cursor-pointer appearance-none pr-12 ${Icon ? "pl-12" : ""} ${value === "" ? "text-muted" : ""}`}
        >
          <option value="" disabled>
            {placeholder}
          </option>
          {options.map((option) => (
            <option key={option} value={option}>
              {option}
            </option>
          ))}
        </select>
        <ChevronDown aria-hidden className="pointer-events-none absolute top-1/2 right-4 size-5 -translate-y-1/2 text-muted" />
      </div>
    </div>
  );
}
