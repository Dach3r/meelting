import { useId, type InputHTMLAttributes } from "react";
import { Input } from "../atoms/Input";
import { Label } from "../atoms/Label";

type Props = InputHTMLAttributes<HTMLInputElement> & {
  label: string;
  error?: string | null;
};

export function FormField({ label, error, ...input }: Props) {
  const id = useId();
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <div
        className={`flex min-h-16 items-center rounded-2xl border bg-surface px-4 py-2 transition-colors focus-within:bg-raised ${error ? "border-error" : "border-transparent"}`}
      >
        <Input id={id} aria-invalid={error ? true : undefined} {...input} />
      </div>
    </div>
  );
}
