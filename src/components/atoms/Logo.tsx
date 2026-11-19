export function Logo() {
  return (
    <span className="flex items-center gap-3 font-brand text-3xl font-bold tracking-tight text-ink">
      <svg
        viewBox="0 0 32 32"
        className="size-8 shrink-0"
        fill="none"
        aria-hidden="true"
      >
        <path
          d="M4 13v6M10 8v16M16 3v26M22 9v14M28 13v6"
          stroke="currentColor"
          strokeWidth="3"
          strokeLinecap="round"
        />
      </svg>
      <span className="lowercase group-data-collapsed/rail:hidden">
        meelting
      </span>
    </span>
  );
}
