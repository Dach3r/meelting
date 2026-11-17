import { Minus, Plus } from "lucide-react";
import { useI18n } from "../../i18n/I18nProvider";

export const minZoom = 0.6;
export const maxZoom = 1.4;
const step = 0.1;

type Props = { zoom: number; onChange: (zoom: number) => void };

export function ZoomControls({ zoom, onChange }: Props) {
  const { t } = useI18n();
  const move = (delta: number) => onChange(Math.round((zoom + delta) * 10) / 10);
  const button =
    "grid size-9 cursor-pointer place-items-center rounded-lg text-muted transition-colors enabled:hover:bg-primary enabled:hover:text-ink disabled:cursor-not-allowed disabled:opacity-40 focus-visible:outline-2 focus-visible:outline-ink";
  return (
    <div className="flex items-center gap-1 rounded-xl bg-raised p-1">
      <button type="button" aria-label={t("teams.zoom.out")} disabled={zoom <= minZoom} onClick={() => move(-step)} className={button}>
        <Minus className="size-4" aria-hidden="true" />
      </button>
      <button
        type="button"
        aria-label={t("teams.zoom.reset")}
        onClick={() => onChange(1)}
        className="min-w-14 cursor-pointer rounded-lg px-2 py-1.5 text-center text-sm text-muted transition-colors hover:bg-primary hover:text-ink focus-visible:outline-2 focus-visible:outline-ink"
      >
        {Math.round(zoom * 100)}%
      </button>
      <button type="button" aria-label={t("teams.zoom.in")} disabled={zoom >= maxZoom} onClick={() => move(step)} className={button}>
        <Plus className="size-4" aria-hidden="true" />
      </button>
    </div>
  );
}
