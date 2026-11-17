import { useI18n } from "../../i18n/I18nProvider";
import type { WorkspaceSwitcherProps } from "../molecules/WorkspaceSwitcher";
import { Rail } from "../organisms/Rail";
import { Topbar } from "../organisms/Topbar";

export function Workspace(props: WorkspaceSwitcherProps) {
  const { t } = useI18n();
  return (
    <div className="flex h-dvh bg-rail">
      <Rail />
      <div className="flex min-w-0 flex-1 flex-col">
        <Topbar {...props} />
        <main className="grid flex-1 place-content-center gap-1 bg-shell p-8 text-center">
          <h1 className="text-2xl font-bold">{t("home.empty.title")}</h1>
          <p className="text-muted">{t("home.empty.body")}</p>
        </main>
      </div>
    </div>
  );
}
