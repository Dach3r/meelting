import type { LucideIcon } from "lucide-react";
import { useId, type InputHTMLAttributes } from "react";
import { Input } from "../atoms/Input";
import { Label } from "../atoms/Label";

type Props = InputHTMLAttributes<HTMLInputElement> & {
  label: string;
  icon?: LucideIcon;
  error?: string | null;
};

export function FormField({ label, icon: Icon, error, ...input }: Props) {
  const id = useId();
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <div
        className={`flex min-h-16 items-center gap-3 rounded-2xl border bg-surface py-2 ${Icon ? "pl-2.5" : "pl-4"} pr-4 transition-colors focus-within:bg-raised ${error ? "border-error" : "border-transparent"}`}
      >
        {Icon && (
          <span className="grid size-10 shrink-0 place-items-center rounded-xl bg-primary text-ink">
            <Icon className="size-5" aria-hidden="true" />
          </span>
        )}
        <Input id={id} aria-invalid={error ? true : undefined} {...input} />
      </div>
    </div>
  );
}
