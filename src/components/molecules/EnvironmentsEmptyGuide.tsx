import { FolderGit2, GitBranch, ListChecks, MoveRight } from "lucide-react";
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
      <ol className="grid w-full max-w-[760px] grid-cols-1 gap-3 md:grid-cols-3">
        {steps.map(({ key, icon: Icon }, index) => (
          <li
            key={key}
            className="relative flex flex-col gap-4 rounded-2xl border border-line bg-raised p-4"
          >
            <div className="flex items-center justify-between">
              <span
                aria-hidden
                className="grid size-10 place-items-center rounded-xl bg-hi text-ink"
              >
                <Icon className="size-5" strokeWidth={1.6} />
              </span>
              <span
                aria-hidden
                className="font-mono text-xs font-semibold text-muted"
              >
                {index + 1}
              </span>
            </div>
            <div>
              <p className="text-[13px] font-bold text-ink">
                {t(`files.guide.${key}.title`)}
              </p>
              <p className="text-xs text-muted">
                {t(`files.guide.${key}.body`)}
              </p>
            </div>
            {index < steps.length - 1 && (
              <MoveRight
                aria-hidden
                className="absolute top-1/2 -right-[11px] z-10 hidden size-5 -translate-y-1/2 rounded-full bg-pill p-0.5 text-muted md:block"
                strokeWidth={1.6}
              />
            )}
          </li>
        ))}
      </ol>
    </aside>
  );
}
