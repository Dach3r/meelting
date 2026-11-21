export function CloseIcon() {
  return (
    <svg
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.7}
      strokeLinecap="round"
      aria-hidden
      className="size-5"
    >
      <path d="M5 5l10 10M15 5L5 15" />
    </svg>
  );
}

export function FolderTileIcon({
  className = "h-20 w-24",
}: {
  className?: string;
}) {
  return (
    <svg
      className={className}
      viewBox="0 0 116 96"
      fill="currentColor"
      aria-hidden
    >
      <path d="M6 18a8 8 0 0 1 8-8h26l10 10h52a8 8 0 0 1 8 8v54a8 8 0 0 1-8 8H14a8 8 0 0 1-8-8z" />
    </svg>
  );
}
