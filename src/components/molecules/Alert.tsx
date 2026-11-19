export function Alert({ message }: { message: string }) {
  return (
    <div
      role="alert"
      className="flex items-start gap-3 rounded-2xl border border-error/25 bg-error/10 px-4 py-3 text-sm text-error"
    >
      <svg
        viewBox="0 0 20 20"
        fill="none"
        stroke="currentColor"
        strokeWidth={1.7}
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden
        className="mt-px size-5 shrink-0"
      >
        <circle cx="10" cy="10" r="7.5" />
        <path d="M10 6.2v4.6M10 13.6v.01" />
      </svg>
      <p className="font-medium leading-5">{message}</p>
    </div>
  );
}
