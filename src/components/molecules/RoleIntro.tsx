import { useI18n } from "../../i18n/I18nProvider";

export function RoleIntro({ role }: { role: string }) {
  const { t } = useI18n();
  const key = `teams.agent.roleProfiles.${role.toLowerCase().replace(/[^a-z]+/g, "_")}`;
  return (
    <div className="grid gap-2.5 pt-0.5 pr-3 pb-3.5 pl-[3.75rem]">
      <q className="text-[0.95rem] leading-relaxed text-ink">{t(`${key}.bio`)}</q>
      <ul className="flex flex-wrap gap-1.5">
        {t(`${key}.skills`)
          .split(", ")
          .map((skill) => (
            <li key={skill} className="rounded-full bg-hi px-2.5 py-1 text-xs font-medium text-muted">
              {skill}
            </li>
          ))}
      </ul>
    </div>
  );
}
