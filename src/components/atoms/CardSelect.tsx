type Props = { label: string; selected: boolean; onSelect: () => void };

const handle = "absolute size-2.5 rounded-xs border-2 border-accent bg-surface";

export function CardSelect({ label, selected, onSelect }: Props) {
  return (
    <>
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
      {selected && (
        <>
          <span aria-hidden="true" className={`${handle} -top-1.5 -left-1.5`} />
          <span aria-hidden="true" className={`${handle} -top-1.5 -right-1.5`} />
          <span aria-hidden="true" className={`${handle} -bottom-1.5 -left-1.5`} />
          <span aria-hidden="true" className={`${handle} -right-1.5 -bottom-1.5`} />
        </>
      )}
    </>
  );
}
