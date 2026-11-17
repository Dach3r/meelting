import { ChevronDown, ChevronUp } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";
import { Avatar } from "../atoms/Avatar";

type Props = {
  seed: string;
  name: string;
  role: string;
  asHeading?: boolean;
  count?: number;
  expanded?: boolean;
  onToggle?: () => void;
};

export function OrgCard({ seed, name, role, asHeading = false, count = 0, expanded = true, onToggle }: Props) {
  const { t } = useI18n();
  const Name = asHeading ? "h2" : "p";
  return (
    <div className="relative w-49 pt-8">
      <Avatar seed={seed} label={name} className="absolute top-0 left-1/2 size-16 -translate-x-1/2 rounded-xl text-2xl" />
      <div className="grid min-h-24 justify-items-center gap-0.5 rounded-xl border border-line bg-raised px-3 pt-10 pb-2 text-center">
        <Name className="max-w-full truncate font-bold text-ink">{name}</Name>
        <p className="text-sm text-muted">{role}</p>
        {count > 0 && (
          <button
            type="button"
            aria-expanded={expanded}
            aria-label={`${t("teams.chart.toggle")} ${name}`}
            onClick={onToggle}
            className="mt-auto flex cursor-pointer items-center gap-0.5 justify-self-end rounded-md px-1 text-sm text-ink transition-colors hover:bg-primary focus-visible:outline-2 focus-visible:outline-ink"
          >
            {count}
            {expanded ? (
              <ChevronUp className="size-4" aria-hidden="true" />
            ) : (
              <ChevronDown className="size-4" aria-hidden="true" />
            )}
          </button>
        )}
      </div>
    </div>
  );
}
