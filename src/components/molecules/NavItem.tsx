import type { LucideIcon } from "lucide-react";
import { useId, useState, type CSSProperties } from "react";
import { useI18n } from "../../i18n/I18nProvider";

const flyoutClasses =
  "absolute top-1/2 left-full -ml-3 flex h-10 -translate-y-1/2 animate-flyout items-center rounded-xl pr-4 pl-7 text-base font-semibold whitespace-nowrap motion-reduce:animate-none";

function Flyout({
  label,
  current,
  enabled,
  anchor,
}: {
  label: string;
  current: boolean;
  enabled: boolean;
  anchor: DOMRect;
}) {
  const filterId = `nav-goo-${useId().replace(/:/g, "")}`;
  const box: CSSProperties = {
    top: anchor.top,
    left: anchor.left,
    width: anchor.width,
    height: anchor.height,
  };
  const fill = current
    ? "bg-[color-mix(in_srgb,var(--color-ink)_12%,var(--color-rail))]"
    : "bg-[color-mix(in_srgb,var(--color-ink)_6%,var(--color-rail))]";
  return (
    <>
      <svg className="absolute size-0" aria-hidden="true" focusable="false">
        <filter
          id={filterId}
          x="-20%"
          y="-50%"
          width="600%"
          height="200%"
          colorInterpolationFilters="sRGB"
        >
          <feGaussianBlur in="SourceGraphic" stdDeviation="5" result="blur" />
          <feColorMatrix
            in="blur"
            values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 20 -9"
          />
        </filter>
      </svg>
      <div
        className="pointer-events-none fixed"
        style={{ ...box, filter: `url(#${filterId})` }}
        aria-hidden="true"
      >
        <div className={`absolute inset-0 rounded-2xl ${fill}`} />
        <div className={`${flyoutClasses} ${fill}`}>
          <span className="invisible">{label}</span>
        </div>
      </div>
      <div
        className="pointer-events-none fixed z-10"
        style={box}
        aria-hidden="true"
      >
        <span
          className={`${flyoutClasses} ${enabled ? "text-ink" : "text-muted/50"}`}
        >
          {label}
        </span>
      </div>
    </>
  );
}

export function NavItem({
  label,
  icon: Icon,
  collapsed,
  current = false,
  onSelect,
}: {
  label: string;
  icon: LucideIcon;
  collapsed: boolean;
  current?: boolean;
  onSelect?: () => void;
}) {
  const { t } = useI18n();
  const [anchor, setAnchor] = useState<DOMRect | null>(null);
  const classes =
    "flex h-14 items-center gap-4 rounded-2xl px-4 text-base font-semibold group-data-collapsed/rail:justify-center group-data-collapsed/rail:px-0";
  const content = (
    <>
      <Icon className="size-6 shrink-0 group-data-collapsed/rail:relative group-data-collapsed/rail:z-20" aria-hidden="true" />
      <span className="whitespace-nowrap group-data-collapsed/rail:sr-only">
        {label}
      </span>
    </>
  );
  let item;
  if (current) {
    const currentClasses = `${classes} bg-ink/12 text-ink`;
    item = onSelect ? (
      <button
        type="button"
        aria-current="page"
        onClick={onSelect}
        className={`${currentClasses} w-full cursor-pointer focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ink`}
      >
        {content}
      </button>
    ) : (
      <span aria-current="page" className={currentClasses}>
        {content}
      </span>
    );
  } else if (onSelect) {
    item = (
      <button
        type="button"
        onClick={onSelect}
        className={`${classes} w-full cursor-pointer hover:bg-ink/6 hover:text-ink focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ink`}
      >
        {content}
      </button>
    );
  } else {
    item = (
      <span
        aria-disabled="true"
        title={t("home.soon")}
        className={`${classes} cursor-not-allowed text-muted/50`}
      >
        {content}
      </span>
    );
  }
  return (
    <div
      onPointerEnter={(event) =>
        collapsed && setAnchor(event.currentTarget.getBoundingClientRect())
      }
      onPointerLeave={() => setAnchor(null)}
    >
      {item}
      {collapsed && anchor && (
        <Flyout
          label={label}
          current={current}
          enabled={Boolean(onSelect) || current}
          anchor={anchor}
        />
      )}
    </div>
  );
}
