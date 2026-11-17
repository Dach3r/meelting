import type { ButtonHTMLAttributes } from "react";

const variants = {
  primary: "border-ink bg-ink text-surface enabled:hover:bg-ink/85",
  ghost: "border-transparent bg-transparent text-muted enabled:hover:bg-pill-hover enabled:hover:text-ink",
};

const sizes = {
  default: "min-h-16 w-full rounded-2xl px-6 text-lg",
  compact: "min-h-11 rounded-xl px-4 text-sm",
  pill: "min-h-11 rounded-full px-5 text-sm",
};

type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: keyof typeof variants;
  size?: keyof typeof sizes;
};

export function Button({ variant = "primary", size = "default", className = "", ...rest }: Props) {
  return (
    <button
      className={`inline-flex cursor-pointer items-center justify-center gap-3 border font-semibold transition-colors duration-200 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink disabled:cursor-not-allowed disabled:opacity-60 ${sizes[size]} ${variants[variant]} ${className}`}
      {...rest}
    />
  );
}
