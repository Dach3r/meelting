import { Check, Search, type LucideIcon } from "lucide-react";
import { useEffect, useId, useState, type ReactNode } from "react";
import { Avatar } from "./Avatar";

export type SelectGroup = {
  label: string;
  icon: LucideIcon;
  options: string[];
};

type Props = {
  label: string;
  searchPlaceholder: string;
  emptyMessage: string;
  value: string;
  groups: SelectGroup[];
  onChange: (value: string) => void;
  renderDetail: (value: string) => ReactNode;
  autoFocus?: boolean;
};

export function SelectField({ label, searchPlaceholder, emptyMessage, value, groups, onChange, renderDetail, autoFocus }: Props) {
  const id = useId();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(() => Math.max(groups.flatMap((group) => group.options).indexOf(value), 0));
  const visible = groups
    .map((group) => ({ ...group, options: group.options.filter((option) => option.toLowerCase().includes(query.trim().toLowerCase())) }))
    .filter((group) => group.options.length > 0);
  const flat = visible.flatMap((group) => group.options);

  useEffect(() => {
    document.getElementById(`${id}-${active}`)?.scrollIntoView({ block: "nearest" });
  }, [active, id]);

  function onKeyDown(event: React.KeyboardEvent) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      if (flat.length > 0) setActive((current) => (current + step + flat.length) % flat.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (flat[active]) onChange(flat[active]);
    }
  }

  let index = -1;
  return (
    <div className="grid gap-2">
      <span id={`${id}-label`} className="text-sm font-medium text-muted">
        {label}
      </span>
      <div className="overflow-hidden rounded-2xl border border-line bg-raised" onKeyDown={onKeyDown}>
        <div className="relative border-b border-line">
          <Search aria-hidden className="pointer-events-none absolute top-1/2 left-4 size-4 -translate-y-1/2 text-muted" />
          <input
            type="text"
            role="combobox"
            aria-expanded
            aria-controls={`${id}-list`}
            aria-activedescendant={flat.length > 0 ? `${id}-${active}` : undefined}
            value={query}
            autoFocus={autoFocus}
            onChange={(event) => {
              setQuery(event.target.value);
              setActive(0);
            }}
            placeholder={searchPlaceholder}
            aria-label={searchPlaceholder}
            className="w-full bg-transparent py-3.5 pr-4 pl-10 text-base text-ink outline-none placeholder:text-muted"
          />
        </div>
        <div id={`${id}-list`} role="listbox" aria-labelledby={`${id}-label`} className="max-h-96 overflow-y-auto overscroll-contain p-1.5">
          {flat.length === 0 && <p className="px-3 py-4 text-center text-sm text-muted">{emptyMessage}</p>}
          {visible.map(({ label: groupLabel, icon: GroupIcon, options }) => (
            <div key={groupLabel} role="group" aria-label={groupLabel}>
              <p aria-hidden className="flex items-center gap-2 px-3 pt-3 pb-1.5 text-xs font-semibold tracking-wider text-muted uppercase">
                <GroupIcon className="size-3.5" />
                {groupLabel}
              </p>
              {options.map((option) => {
                const position = ++index;
                return (
                  <div key={option} className={`rounded-xl ${option === value ? "bg-surface ring-1 ring-line" : ""}`}>
                    <div
                      id={`${id}-${position}`}
                      role="option"
                      aria-selected={option === value}
                      onMouseEnter={() => setActive(position)}
                      onMouseDown={(event) => event.preventDefault()}
                      onClick={() => onChange(option)}
                      className={`flex cursor-pointer items-center gap-3 rounded-xl p-2.5 text-base text-ink ${option === value ? "font-bold" : position === active ? "bg-hi/50" : ""}`}
                    >
                      <Avatar seed={option} label={option} className="size-9 rounded-full text-sm" />
                      <span className="flex-1">{option}</span>
                      {option === value && <Check aria-hidden className="size-4 text-accent" />}
                    </div>
                    {option === value && renderDetail(option)}
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
