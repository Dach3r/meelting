import { Check, FileText, User, Users, type LucideIcon } from "lucide-react";
import { useId, useState, type FormEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { useAgentRoles } from "../../hooks/useAgentRoles";
import type { AgentDetail, AgentEdit, Team } from "../../hooks/useTeams";
import { Avatar } from "../atoms/Avatar";
import { Button } from "../atoms/Button";
import { Field } from "../atoms/Field";
import { roleGroupIcons } from "../atoms/roleGroupIcons";
import { SelectField } from "../atoms/SelectField";
import { Alert } from "../molecules/Alert";
import { TeamPicker } from "../molecules/TeamPicker";

type Props = {
  agent: AgentDetail;
  teams: Team[];
  busy: boolean;
  errorMessage: string | null;
  onSubmit: (edit: AgentEdit) => Promise<boolean>;
  onClose: () => void;
};

const tabs = [
  { key: "identity", icon: User, fields: ["name", "role"] },
  { key: "team", icon: Users, fields: ["team_id"] },
  { key: "behavior", icon: FileText, fields: ["description", "instructions"] },
] as const satisfies readonly { key: string; icon: LucideIcon; fields: readonly (keyof AgentEdit)[] }[];

const pickEdit = ({ team_id, name, role, description, instructions }: AgentEdit): AgentEdit => ({
  team_id,
  name,
  role,
  description,
  instructions,
});

export function AgentEditForm({ agent, teams, busy, errorMessage, onSubmit, onClose }: Props) {
  const { t } = useI18n();
  const panelId = useId();
  const roleGroups = useAgentRoles().map(({ group, roles }) => ({
    id: group,
    label: t(`teams.agent.roleGroups.${group}`),
    icon: roleGroupIcons[group],
    options: roles,
  }));
  const [tab, setTab] = useState(0);
  const [saved, setSaved] = useState<AgentEdit>(() => pickEdit(agent));
  const [draft, setDraft] = useState<AgentEdit>(saved);
  const set = (field: keyof AgentEdit) => (value: string) => setDraft({ ...draft, [field]: value });
  const current = tabs[tab];
  const changed = current.fields.some((field) => draft[field] !== saved[field]);
  const valid = current.fields.every((field) => field === "description" || field === "instructions" || draft[field].trim() !== "");

  async function submit(event: FormEvent) {
    event.preventDefault();
    const edit = { ...saved };
    for (const field of current.fields) edit[field] = draft[field];
    if (await onSubmit(edit)) setSaved(edit);
  }

  return (
    <div className="grid min-h-[44rem] sm:grid-cols-[17rem_1fr]">
      <div className="grid content-start gap-4 bg-raised/50 sm:border-r sm:border-line px-4 py-6 sm:px-5 sm:py-10">
        <div className="flex items-center gap-3 rounded-2xl border border-line bg-surface p-3">
          <Avatar seed={saved.name} label={saved.name} className="size-10 rounded-full text-base" />
          <div className="min-w-0">
            <p className="truncate text-sm font-semibold text-ink">{saved.name}</p>
            <p className="truncate text-xs text-muted">{saved.role}</p>
          </div>
        </div>
        <div role="tablist" aria-orientation="vertical" aria-label={t("teams.agent.edit")} className="overflow-hidden rounded-2xl border border-line bg-surface">
          {tabs.map(({ key, icon: TabIcon }, index) => (
            <button
              key={key}
              type="button"
              role="tab"
              id={`${panelId}-${key}-tab`}
              aria-selected={index === tab}
              aria-controls={`${panelId}-panel`}
              onClick={() => setTab(index)}
              className={`flex w-full cursor-pointer items-center gap-4 px-4 py-4 text-left not-last:border-b not-last:border-line focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ink ${index === tab ? "bg-hi" : "hover:bg-hi/50"}`}
            >
              <span
                className={`grid size-7 shrink-0 place-items-center rounded-full ${index === tab ? "bg-ink text-surface" : "border border-muted text-muted"}`}
              >
                <TabIcon aria-hidden className="size-4" />
              </span>
              <span className={`text-sm ${index === tab ? "font-semibold text-ink" : "font-medium text-muted"}`}>
                {t(`teams.agent.${key}`)}
              </span>
            </button>
          ))}
        </div>
      </div>
      <form
        onSubmit={submit}
        role="tabpanel"
        id={`${panelId}-panel`}
        aria-labelledby={`${panelId}-${current.key}-tab`}
        className="flex flex-col gap-10 p-6 sm:px-10 sm:pt-10 sm:pb-6"
      >
        <div className="grid gap-2">
          <h3 className="text-2xl font-bold text-ink">{t(`teams.agent.${current.key}Title`)}</h3>
          <p className="text-sm text-muted">{t(`teams.agent.${current.key}Subtitle`)}</p>
        </div>
        <div className="flex flex-1 flex-col gap-7">
          {current.key === "identity" && (
            <>
              <Field label={t("teams.agent.name")} value={draft.name} onChange={set("name")} maxLength={80} icon={User} autoFocus />
              <SelectField
                label={t("teams.agent.role")}
                searchPlaceholder={t("teams.agent.roleSearch")}
                emptyMessage={t("teams.agent.roleEmpty")}
                value={draft.role}
                groups={roleGroups}
                onChange={set("role")}
              />
            </>
          )}
          {current.key === "team" && (
            <TeamPicker teams={teams} currentId={saved.team_id} value={draft.team_id} onChange={set("team_id")} />
          )}
          {current.key === "behavior" && (
            <>
              <Field
                label={t("teams.agent.description")}
                value={draft.description}
                onChange={set("description")}
                maxLength={500}
                multiline
                rows={2}
                hint={t("teams.agent.descriptionHint")}
                placeholder={t("teams.agent.descriptionPlaceholder")}
              />
              <Field
                label={t("teams.agent.instructions")}
                value={draft.instructions}
                onChange={set("instructions")}
                maxLength={10000}
                multiline
                grow
                hint={t("teams.agent.instructionsHint")}
                placeholder={t("teams.agent.instructionsPlaceholder")}
              />
            </>
          )}
          {errorMessage && <Alert message={errorMessage} />}
        </div>
        <div className="flex items-center justify-end gap-2">
          <Button type="button" variant="ghost" size="pill" onClick={onClose}>
            {t("teams.close")}
          </Button>
          <Button type="submit" size="pill" disabled={busy || !changed || !valid}>
            <Check aria-hidden className="size-4" />
            {t("teams.agent.save")}
          </Button>
        </div>
      </form>
    </div>
  );
}
