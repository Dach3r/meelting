function hueOf(seed: string) {
  let hue = 0;
  for (const char of seed) hue = (hue * 31 + char.charCodeAt(0)) % 360;
  return hue;
}

type Props = { seed: string; label: string; className?: string };

export function Avatar({ seed, label, className = "" }: Props) {
  const hue = hueOf(seed);
  return (
    <span
      aria-hidden="true"
      style={{ backgroundColor: `hsl(${hue} 30% 28%)`, color: `hsl(${hue} 70% 85%)` }}
      className={`grid shrink-0 place-items-center font-bold ${className}`}
    >
      {label.trim().charAt(0).toUpperCase()}
    </span>
  );
}
