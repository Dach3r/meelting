type Props = { label: string; selected: boolean; onSelect: () => void };

export function CardSelect({ label, selected, onSelect }: Props) {
  return (
    <button
      type="button"
      aria-label={label}
      aria-pressed={selected}
      onClick={(event) => {
        event.stopPropagation();
        onSelect();
      }}
      className="absolute inset-0 cursor-pointer rounded-2xl focus-visible:outline-2 focus-visible:outline-accent"
    />
  );
}
