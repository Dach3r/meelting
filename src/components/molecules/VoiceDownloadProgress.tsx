import { useI18n } from "../../i18n/I18nProvider";
import { translateWith } from "../../i18n/translate";

type Props = {
  downloaded: number;
  total: number;
};

const megabytes = (bytes: number) => Math.round(bytes / 1_000_000);

export function VoiceDownloadProgress({ downloaded, total }: Props) {
  const { t, locale } = useI18n();
  const percent = total > 0 ? Math.min(100, (downloaded * 100) / total) : 0;
  const verifying = percent >= 100;

  return (
    <div className="grid w-full gap-3" aria-live="polite">
      <div
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.floor(percent)}
        className="h-3 overflow-hidden rounded-full bg-raised"
      >
        <div
          className="h-full rounded-full bg-ink transition-[width] duration-200"
          style={{ width: `${percent}%` }}
        />
      </div>
      <p className="text-sm font-medium text-muted tabular-nums">
        {verifying
          ? t("rooms.voice.verifying")
          : translateWith(locale, "rooms.voice.progress", {
              done: megabytes(downloaded),
              total: megabytes(total),
            })}
      </p>
    </div>
  );
}
