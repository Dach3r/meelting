import type { InputHTMLAttributes } from "react";

export function Input({ className = "", ...rest }: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      className={`h-10 w-full min-w-0 border-0 bg-transparent p-0 text-lg font-medium text-ink outline-none placeholder:text-muted ${className}`}
      {...rest}
    />
  );
}
