import type { ReactNode } from "react";

export function Alert({
  message,
  action,
}: {
  message: string;
  action?: ReactNode;
}) {
  return (
    <div
      role="alert"
      className="flex items-center gap-3 rounded-2xl border border-error/25 bg-error/10 px-4 py-3 text-sm text-error"
    >
      <svg
        viewBox="0 0 20 20"
        fill="none"
        stroke="currentColor"
        strokeWidth={1.7}
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden
        className="size-5 shrink-0"
      >
        <circle cx="10" cy="10" r="7.5" />
        <path d="M10 6.2v4.6M10 13.6v.01" />
      </svg>
      <p className="min-w-0 flex-1 font-medium leading-5">{message}</p>
      {action}
    </div>
  );
}
