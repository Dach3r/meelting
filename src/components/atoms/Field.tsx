import { useId, type InputHTMLAttributes } from "react";

type Props = Omit<InputHTMLAttributes<HTMLInputElement>, "onChange"> & {
  label: string;
  value: string;
  onChange: (value: string) => void;
  multiline?: boolean;
};

const classes =
  "w-full min-w-0 rounded-xl border border-line bg-surface px-4 py-3 text-base text-ink outline-none transition-colors placeholder:text-muted focus:border-ink";

export function Field({ label, value, onChange, multiline = false, ...rest }: Props) {
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
          className={classes}
        />
      ) : (
        <input id={id} value={value} onChange={(event) => onChange(event.target.value)} className={classes} {...rest} />
      )}
    </div>
  );
}
