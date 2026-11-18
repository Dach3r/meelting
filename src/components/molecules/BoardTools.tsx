import { Hand, MousePointer2 } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";

export type BoardTool = "select" | "pan";

type Props = { tool: BoardTool; onChange: (tool: BoardTool) => void };

export function BoardTools({ tool, onChange }: Props) {
  const { t } = useI18n();
  const button = (value: BoardTool) =>
    `grid size-10 cursor-pointer place-items-center rounded-xl transition-colors focus-visible:outline-2 focus-visible:outline-ink ${
      tool === value ? "bg-hi text-ink" : "text-muted hover:bg-primary hover:text-ink"
    }`;
  return (
    <div className="flex flex-col gap-1 rounded-2xl border border-line bg-raised p-1.5 shadow-sm">
      <button type="button" aria-pressed={tool === "select"} aria-label={t("teams.board.select")} title={t("teams.board.select")} onClick={() => onChange("select")} className={button("select")}>
        <MousePointer2 className="size-5" aria-hidden="true" />
      </button>
      <button type="button" aria-pressed={tool === "pan"} aria-label={t("teams.board.pan")} title={t("teams.board.pan")} onClick={() => onChange("pan")} className={button("pan")}>
        <Hand className="size-5" aria-hidden="true" />
      </button>
    </div>
  );
}
