import type { ReactNode } from "react";

export function SettingsGroup({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="grid gap-2">
      <p className="px-1 text-xs font-bold tracking-wider text-muted uppercase">
        {title}
      </p>
      <div className="rounded-2xl bg-pill-hover">{children}</div>
    </section>
  );
}
