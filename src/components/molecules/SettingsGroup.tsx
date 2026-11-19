import type { ReactNode } from "react";

export function SettingsGroup({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="grid gap-3">
      <p className="px-1 text-xs font-bold tracking-wider text-muted uppercase">
        {title}
      </p>
      <div className="divide-y divide-line/50 overflow-hidden rounded-2xl border border-line bg-hi/30">
        {children}
      </div>
    </section>
  );
}
