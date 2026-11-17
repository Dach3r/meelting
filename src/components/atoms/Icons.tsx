const props = {
  viewBox: "0 0 20 20",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.7,
  strokeLinecap: "round",
  "aria-hidden": true,
  className: "size-5",
} as const;

export function UsersIcon() {
  return (
    <svg {...props}>
      <circle cx="8" cy="7" r="3" />
      <path d="M2.5 16c0-3 2.5-5 5.5-5s5.5 2 5.5 5" />
      <circle cx="14.5" cy="8" r="2.3" />
      <path d="M14 11.2c2.4 0 4 1.6 4 4" />
    </svg>
  );
}

export function BotIcon() {
  return (
    <svg {...props}>
      <rect x="3" y="7" width="14" height="9" rx="3" />
      <path d="M10 4v3" />
      <circle cx="7.5" cy="11.5" r="1" fill="currentColor" />
      <circle cx="12.5" cy="11.5" r="1" fill="currentColor" />
    </svg>
  );
}

export function CloseIcon() {
  return (
    <svg {...props}>
      <path d="M5 5l10 10M15 5L5 15" />
    </svg>
  );
}
