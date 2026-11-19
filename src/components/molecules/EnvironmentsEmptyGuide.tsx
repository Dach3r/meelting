import { FolderGit2, GitBranch, ListChecks } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";

const steps: { key: "first" | "second" | "third"; icon: LucideIcon }[] = [
  { key: "first", icon: FolderGit2 },
  { key: "second", icon: ListChecks },
  { key: "third", icon: GitBranch },
];

export function EnvironmentsEmptyGuide() {
  const { t } = useI18n();
  return (
    <aside
      aria-label={t("files.guide.title")}
      className="grid min-w-0 flex-1 place-items-center overflow-auto rounded-3xl border border-line bg-pill p-7"
    >
      <ol className="grid w-full max-w-[660px] grid-cols-1 gap-8 md:grid-cols-3 md:gap-0">
        {steps.map(({ key, icon: Icon }, index) => (
          <li
            key={key}
            className="relative flex flex-col items-center gap-4 px-4 text-center"
          >
            {index < steps.length - 1 && (
              <span
                aria-hidden
                className="absolute top-[26px] left-1/2 hidden w-full border-t border-dashed border-line md:block"
              />
            )}
            <span
              aria-hidden
              className="relative z-10 grid size-[52px] place-items-center rounded-2xl border border-line bg-raised text-ink"
            >
              <Icon className="size-6" strokeWidth={1.5} />
              <span className="absolute -top-2 -right-2 grid size-5 place-items-center rounded-full bg-primary font-mono text-[11px] font-semibold">
                {index + 1}
              </span>
            </span>
            <div className="flex flex-col gap-1">
              <p className="text-[13px] font-bold text-ink">
                {t(`files.guide.${key}.title`)}
              </p>
              <p className="text-xs text-muted">
                {t(`files.guide.${key}.body`)}
              </p>
            </div>
          </li>
        ))}
      </ol>
    </aside>
  );
}
