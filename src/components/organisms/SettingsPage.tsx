import {
  Bell,
  FolderGit2,
  Palette,
  SlidersHorizontal,
  Wrench,
  type LucideIcon,
} from "lucide-react";
import { useState, type ReactNode } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Locale, TimeFormat } from "../../i18n/translate";
import type {
  NotificationSetting,
  ReduceMotion,
  Theme,
  UiScale,
} from "../../hooks/useSession";
import { Switch } from "../atoms/Switch";
import { Alert } from "../molecules/Alert";
import { LocaleSwitcher } from "../molecules/LocaleSwitcher";
import { SegmentedControl } from "../molecules/SegmentedControl";
import { SettingsGroup } from "../molecules/SettingsGroup";
import { SettingsRow } from "../molecules/SettingsRow";
import { ThemeSwitcher } from "../molecules/ThemeSwitcher";
import { UiScaleSwitcher } from "../molecules/UiScaleSwitcher";
import { DataFolderSetting } from "./DataFolderSetting";
import { OpenAtLoginSetting } from "./OpenAtLoginSetting";
import { WorktreesRootSetting } from "./WorktreesRootSetting";

type Props = {
  locale: Locale;
  theme: Theme;
  uiScale: UiScale;
  loadingEnabled: boolean;
  timeFormat: TimeFormat;
  reduceMotion: ReduceMotion;
  notifyRunSucceeded: boolean;
  notifyRunFailed: boolean;
  notifySound: boolean;
  busy: boolean;
  errorMessage: string | null;
  onChangeLocale: (locale: Locale) => void;
  onChangeTheme: (theme: Theme) => void;
  onChangeUiScale: (uiScale: UiScale) => void;
  onChangeLoadingEnabled: (loadingEnabled: boolean) => void;
  onChangeTimeFormat: (timeFormat: TimeFormat) => void;
  onChangeReduceMotion: (reduceMotion: ReduceMotion) => void;
  onChangeNotification: (
    setting: NotificationSetting,
    enabled: boolean,
  ) => void;
};

type Section =
  | "general"
  | "appearance"
  | "notifications"
  | "environments"
  | "advanced";

const sectionOrder: Section[] = [
  "general",
  "appearance",
  "notifications",
  "environments",
  "advanced",
];

const sectionIcons: Record<Section, LucideIcon> = {
  general: SlidersHorizontal,
  appearance: Palette,
  notifications: Bell,
  environments: FolderGit2,
  advanced: Wrench,
};

type Group = { key: string; rows: { key: string; node: ReactNode }[] };

export function SettingsPage({
  locale,
  theme,
  uiScale,
  loadingEnabled,
  timeFormat,
  reduceMotion,
  notifyRunSucceeded,
  notifyRunFailed,
  notifySound,
  busy,
  errorMessage,
  onChangeLocale,
  onChangeTheme,
  onChangeUiScale,
  onChangeLoadingEnabled,
  onChangeTimeFormat,
  onChangeReduceMotion,
  onChangeNotification,
}: Props) {
  const { t } = useI18n();
  const [section, setSection] = useState<Section>("general");

  const notificationRow = (
    key: string,
    setting: NotificationSetting,
    enabled: boolean,
  ) => ({
    key,
    node: (
      <SettingsRow
        title={t(`settings.notifications.${key}.title`)}
        description={t(`settings.notifications.${key}.description`)}
      >
        <Switch
          label={t(`settings.notifications.${key}.title`)}
          checked={enabled}
          onChange={(checked) => onChangeNotification(setting, checked)}
          disabled={busy}
        />
      </SettingsRow>
    ),
  });

  const sections: Record<Section, Group[]> = {
    general: [
      {
        key: "languageRegion",
        rows: [
          {
            key: "language",
            node: (
              <SettingsRow
                title={t("settings.language.title")}
                description={t("settings.language.description")}
              >
                <LocaleSwitcher
                  value={locale}
                  onChange={onChangeLocale}
                  disabled={busy}
                />
              </SettingsRow>
            ),
          },
          {
            key: "timeFormat",
            node: (
              <SettingsRow
                title={t("settings.timeFormat.title")}
                description={t("settings.timeFormat.description")}
              >
                <SegmentedControl
                  label={t("settings.timeFormat.title")}
                  options={[
                    { value: "auto", label: t("settings.timeFormat.auto") },
                    { value: "12h", label: t("settings.timeFormat.12h") },
                    { value: "24h", label: t("settings.timeFormat.24h") },
                  ]}
                  value={timeFormat}
                  onChange={onChangeTimeFormat}
                  disabled={busy}
                />
              </SettingsRow>
            ),
          },
        ],
      },
      {
        key: "startup",
        rows: [{ key: "openAtLogin", node: <OpenAtLoginSetting /> }],
      },
    ],
    appearance: [
      {
        key: "theme",
        rows: [
          {
            key: "theme",
            node: (
              <SettingsRow
                title={t("settings.theme.title")}
                description={t("settings.theme.description")}
              >
                <ThemeSwitcher
                  value={theme}
                  onChange={onChangeTheme}
                  disabled={busy}
                />
              </SettingsRow>
            ),
          },
        ],
      },
      {
        key: "interface",
        rows: [
          {
            key: "uiScale",
            node: (
              <SettingsRow
                title={t("settings.uiScale.title")}
                description={t("settings.uiScale.description")}
              >
                <UiScaleSwitcher
                  value={uiScale}
                  onChange={onChangeUiScale}
                  disabled={busy}
                />
              </SettingsRow>
            ),
          },
          {
            key: "loading",
            node: (
              <SettingsRow
                title={t("settings.loading.title")}
                description={t("settings.loading.description")}
              >
                <SegmentedControl
                  label={t("settings.loading.title")}
                  options={[
                    { value: "on", label: t("settings.loading.on") },
                    { value: "off", label: t("settings.loading.off") },
                  ]}
                  value={loadingEnabled ? "on" : "off"}
                  onChange={(selected) =>
                    onChangeLoadingEnabled(selected === "on")
                  }
                  disabled={busy}
                />
              </SettingsRow>
            ),
          },
          {
            key: "reduceMotion",
            node: (
              <SettingsRow
                title={t("settings.reduceMotion.title")}
                description={t("settings.reduceMotion.description")}
              >
                <SegmentedControl
                  label={t("settings.reduceMotion.title")}
                  options={[
                    {
                      value: "system",
                      label: t("settings.reduceMotion.system"),
                    },
                    { value: "on", label: t("settings.reduceMotion.on") },
                    { value: "off", label: t("settings.reduceMotion.off") },
                  ]}
                  value={reduceMotion}
                  onChange={onChangeReduceMotion}
                  disabled={busy}
                />
              </SettingsRow>
            ),
          },
        ],
      },
    ],
    notifications: [
      {
        key: "alerts",
        rows: [
          notificationRow("runSucceeded", "run_succeeded", notifyRunSucceeded),
          notificationRow("runFailed", "run_failed", notifyRunFailed),
        ],
      },
      {
        key: "sound",
        rows: [notificationRow("sound", "sound", notifySound)],
      },
    ],
    environments: [
      {
        key: "location",
        rows: [{ key: "worktrees", node: <WorktreesRootSetting /> }],
      },
    ],
    advanced: [
      {
        key: "data",
        rows: [
          { key: "dataFolder", node: <DataFolderSetting /> },
          {
            key: "providerKeys",
            node: (
              <SettingsRow
                title={t("settings.providerKeys.title")}
                description={t("settings.providerKeys.description")}
              />
            ),
          },
        ],
      },
    ],
  };

  return (
    <div className="grid min-h-0 flex-1 grid-cols-[16rem_minmax(0,1fr)]">
      <nav
        aria-label={t("settings.categories.label")}
        className="flex flex-col border-r border-line"
      >
        {sectionOrder.map((item) => {
          const Icon = sectionIcons[item];
          const active = item === section;
          return (
            <button
              key={item}
              type="button"
              aria-current={active ? "true" : undefined}
              onClick={() => setSection(item)}
              className={`flex cursor-pointer items-center gap-3 border-b border-l-2 border-b-line px-6 py-4 text-left text-sm focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ink ${active ? "border-l-ink bg-hi font-semibold text-ink" : "border-l-transparent font-medium text-muted hover:bg-pill-hover hover:text-ink"}`}
            >
              <span
                className={`grid size-9 shrink-0 place-items-center rounded-lg border ${active ? "border-ink" : "border-line"}`}
              >
                <Icon className="size-4" aria-hidden />
              </span>
              {t(`settings.categories.${item}.title`)}
            </button>
          );
        })}
      </nav>
      <div className="grid min-h-0 w-full content-start gap-6 overflow-y-auto p-8 pl-10">
        {errorMessage && <Alert message={errorMessage} />}
        <div>
          <h2 className="text-xl font-bold text-ink">
            {t(`settings.categories.${section}.title`)}
          </h2>
          <p className="text-sm text-muted">
            {t(`settings.categories.${section}.description`)}
          </p>
        </div>
        {sections[section].map((group) => (
          <SettingsGroup
            key={group.key}
            title={t(`settings.groups.${group.key}`)}
          >
            {group.rows.map((row) => (
              <div key={row.key}>{row.node}</div>
            ))}
          </SettingsGroup>
        ))}
      </div>
    </div>
  );
}
