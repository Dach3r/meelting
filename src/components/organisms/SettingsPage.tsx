import { FolderGit2, Palette, SlidersHorizontal } from "lucide-react";
import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { Locale } from "../../i18n/translate";
import type { Theme, UiScale } from "../../hooks/useSession";
import { Alert } from "../molecules/Alert";
import { LocaleSwitcher } from "../molecules/LocaleSwitcher";
import { SegmentedControl } from "../molecules/SegmentedControl";
import { SettingsRow } from "../molecules/SettingsRow";
import { ThemeSwitcher } from "../molecules/ThemeSwitcher";
import { UiScaleSwitcher } from "../molecules/UiScaleSwitcher";
import { WorktreesRootSetting } from "./WorktreesRootSetting";

type Props = {
  locale: Locale;
  theme: Theme;
  uiScale: UiScale;
  loadingEnabled: boolean;
  busy: boolean;
  errorMessage: string | null;
  onChangeLocale: (locale: Locale) => void;
  onChangeTheme: (theme: Theme) => void;
  onChangeUiScale: (uiScale: UiScale) => void;
  onChangeLoadingEnabled: (loadingEnabled: boolean) => void;
};

const categories = ["general", "appearance", "environments"] as const;
type Category = (typeof categories)[number];

const categoryIcons = {
  general: SlidersHorizontal,
  appearance: Palette,
  environments: FolderGit2,
};

export function SettingsPage({
  locale,
  theme,
  uiScale,
  loadingEnabled,
  busy,
  errorMessage,
  onChangeLocale,
  onChangeTheme,
  onChangeUiScale,
  onChangeLoadingEnabled,
}: Props) {
  const { t } = useI18n();
  const [category, setCategory] = useState<Category>("general");
  return (
    <div className="grid min-h-0 flex-1 grid-cols-[14rem_minmax(0,1fr)] gap-10">
      <nav
        aria-label={t("settings.categories.label")}
        className="flex flex-col gap-1"
      >
        <h1 className="px-3 pb-3 text-2xl font-bold text-ink">
          {t("settings.title")}
        </h1>
        {categories.map((item) => {
          const Icon = categoryIcons[item];
          return (
            <button
              key={item}
              type="button"
              aria-current={item === category ? "true" : undefined}
              onClick={() => setCategory(item)}
              className={`flex cursor-pointer items-center gap-3 rounded-xl px-3 py-2.5 text-left text-sm focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ink ${item === category ? "bg-pill-hover font-semibold text-ink" : "font-medium text-muted hover:bg-pill-hover hover:text-ink"}`}
            >
              <Icon className="size-4 shrink-0" aria-hidden />
              {t(`settings.categories.${item}.title`)}
            </button>
          );
        })}
      </nav>
      <div className="grid w-full content-start gap-6">
        {errorMessage && <Alert message={errorMessage} />}
        <div>
          <h2 className="text-xl font-bold text-ink">
            {t(`settings.categories.${category}.title`)}
          </h2>
          <p className="text-sm text-muted">
            {t(`settings.categories.${category}.description`)}
          </p>
        </div>
        <div className="rounded-2xl bg-pill-hover">
          {category === "general" && (
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
          )}
          {category === "appearance" && (
            <>
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
            </>
          )}
          {category === "environments" && <WorktreesRootSetting />}
        </div>
      </div>
    </div>
  );
}
