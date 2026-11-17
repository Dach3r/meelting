import { useId, type InputHTMLAttributes } from "react";

type Props = Omit<InputHTMLAttributes<HTMLInputElement>, "onChange"> & {
  label: string;
  value: string;
  onChange: (value: string) => void;
  multiline?: boolean;
};

const classes =
  "w-full min-w-0 rounded-[18px] bg-raised px-4 py-3 text-base text-ink outline-none transition-colors placeholder:text-muted focus:bg-hi";

export function Field({ label, value, onChange, multiline = false, ...rest }: Props) {
  const id = useId();
  return (
    <div className="grid gap-2">
      <label htmlFor={id} className="pl-3 text-sm font-medium text-muted">
        {label}
      </label>
      {multiline ? (
        <textarea
          id={id}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          rows={4}
          maxLength={rest.maxLength}
          className={classes}
        />
      ) : (
        <input id={id} value={value} onChange={(event) => onChange(event.target.value)} className={classes} {...rest} />
      )}
    </div>
  );
}
