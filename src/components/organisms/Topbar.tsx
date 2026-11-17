import { WorkspaceSwitcher, type WorkspaceSwitcherProps } from "../molecules/WorkspaceSwitcher";

export function Topbar(props: WorkspaceSwitcherProps) {
  return (
    <header className="relative z-10 flex h-22 shrink-0 items-start gap-3 bg-shell px-8 pt-4 pb-3">
      <WorkspaceSwitcher {...props} />
    </header>
  );
}
