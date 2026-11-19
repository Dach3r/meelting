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
      <div className="divide-y divide-ink/10 overflow-hidden rounded-2xl border border-ink/20 bg-pill-hover">
        {children}
      </div>
    </section>
  );
}
