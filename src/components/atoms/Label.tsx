import type { LabelHTMLAttributes } from "react";

export function Label({ className = "", ...rest }: LabelHTMLAttributes<HTMLLabelElement>) {
  return <label className={`text-base font-semibold text-ink ${className}`} {...rest} />;
}
