import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import type { WorkspaceSwitcherProps } from "../molecules/WorkspaceSwitcher";
import { SwitchingOverlay } from "../molecules/SwitchingOverlay";
import { Rail } from "../organisms/Rail";
import { Topbar } from "../organisms/Topbar";

const storageKey = "sidebar";
const minSwitchMs = 600;

function storedCollapsed() {
  try {
    return localStorage.getItem(storageKey) === "collapsed";
  } catch {
    return false;
  }
}

type WorkspaceProps = WorkspaceSwitcherProps & {
  errorMessage: string | null;
  onDismissError: () => void;
};

export function Workspace({ errorMessage, onDismissError, ...props }: WorkspaceProps) {
  const { t } = useI18n();
  const [collapsed, setCollapsed] = useState(storedCollapsed);
  const [animated, setAnimated] = useState(false);
  const [switching, setSwitching] = useState<{ name: string; failed: boolean } | null>(null);

  async function switchWorkspace(workspaceId: string) {
    const target = props.workspaces.find((workspace) => workspace.id === workspaceId);
    setSwitching({ name: target?.name ?? "", failed: false });
    const [switched] = await Promise.all([
      props.onSwitch(workspaceId),
      new Promise((resolve) => setTimeout(resolve, minSwitchMs)),
    ]);
    setSwitching(switched ? null : { name: target?.name ?? "", failed: true });
    return switched;
  }

  function toggle() {
    const next = !collapsed;
    setAnimated(true);
    setCollapsed(next);
    try {
      localStorage.setItem(storageKey, next ? "collapsed" : "expanded");
    } catch {
      return;
    }
  }

  return (
    <div
      data-collapsed={collapsed || undefined}
      className={`group/rail relative grid h-dvh grid-cols-[19rem_minmax(0,1fr)] gap-3 overflow-hidden bg-rail p-6 data-collapsed:grid-cols-[4.5rem_minmax(0,1fr)] data-collapsed:pl-3 ${animated ? "transition-[grid-template-columns] duration-300 ease-in-out" : ""}`}
    >
      <Rail collapsed={collapsed} onToggle={toggle} />
      <div className="flex min-h-0 min-w-0 flex-col overflow-hidden rounded-3xl bg-shell">
        <Topbar {...props} onSwitch={switchWorkspace} />
        <main className="flex min-h-0 flex-1 flex-col px-8 pb-8">
          <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-3xl bg-surface">
            <div className="flex min-h-0 flex-1 flex-col overflow-y-auto p-8">
              <div className="mx-auto grid min-h-0 w-full max-w-4xl flex-1 place-content-center gap-1 text-center">
                <h1 className="text-2xl font-bold">{t("home.empty.title")}</h1>
                <p className="text-muted">{t("home.empty.body")}</p>
              </div>
            </div>
          </div>
        </main>
      </div>
      {switching && (
        <SwitchingOverlay
          workspaceName={switching.name}
          errorMessage={switching.failed ? errorMessage : null}
          onDismiss={() => {
            setSwitching(null);
            onDismissError();
          }}
        />
      )}
    </div>
  );
}
