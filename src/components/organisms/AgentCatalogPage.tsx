import { Briefcase, ChartColumn, Code, Package, PenLine, Search, type LucideIcon } from "lucide-react";
import { useState } from "react";
import { useI18n } from "../../i18n/I18nProvider";
import { useAgentRoles } from "../../hooks/useAgentRoles";
import { useAgentTemplates, type AgentTemplate } from "../../hooks/useAgentTemplates";
import type { AgentDraft, Team } from "../../hooks/useTeams";
import { translateError } from "../../i18n/translate";
import { Avatar } from "../atoms/Avatar";
import { Button } from "../atoms/Button";
import { Alert } from "../molecules/Alert";
import { AgentTemplateDetail } from "../molecules/AgentTemplateDetail";
import { Modal } from "../molecules/Modal";
import { SegmentedControl } from "../molecules/SegmentedControl";
import { AgentForm } from "./AgentForm";
import { HireWizard } from "./HireWizard";

type Props = {
  team: Team;
  busy: boolean;
  actionErrorMessage: string | null;
  onCreateAgent: (draft: AgentDraft) => Promise<boolean>;
  onClearActionError: () => void;
  onBack: () => void;
};

const roleGroupIcons: Record<string, LucideIcon> = {
  product: Package,
  engineering: Code,
  data: ChartColumn,
  design_support: PenLine,
  business: Briefcase,
};

type Dialog = { kind: "detail" | "hire"; template: AgentTemplate } | { kind: "custom" } | null;

export function AgentCatalogPage({ team, busy, actionErrorMessage, onCreateAgent, onClearActionError, onBack }: Props) {
  const { t, locale } = useI18n();
  const { templates, errorKey } = useAgentTemplates(locale);
  const rolesByGroup = useAgentRoles();
  const roleGroups = rolesByGroup.map(({ group, roles }) => ({
    label: t(`teams.agent.roleGroups.${group}`),
    icon: roleGroupIcons[group],
    options: roles,
  }));
  const [dialog, setDialog] = useState<Dialog>(null);
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("all");
  const groupOf = (template: AgentTemplate) => rolesByGroup.find(({ roles }) => roles.includes(template.role))?.group ?? "";
  const needle = query.trim().toLowerCase();
  const visibleTemplates = templates.filter(
    (template) =>
      (category === "all" || groupOf(template) === category) &&
      [template.name, template.role, template.description].some((text) => text.toLowerCase().includes(needle)),
  );

  function open(next: NonNullable<Dialog>) {
    onClearActionError();
    setDialog(next);
  }

  function close() {
    onClearActionError();
    setDialog(null);
  }

  async function create(draft: AgentDraft) {
    const created = await onCreateAgent(draft);
    if (created) onBack();
    return created;
  }

  return (
    <div className="grid w-full content-start gap-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-xl font-bold">{t("teams.catalog.title")}</h2>
          <p className="text-muted">{`${t("teams.catalog.team")} ${team.name}`}</p>
        </div>
        <div className="flex gap-3">
          <Button variant="ghost" size="pill" onClick={onBack}>
            {t("teams.catalog.back")}
          </Button>
          <Button size="pill" onClick={() => open({ kind: "custom" })}>
            {t("teams.catalog.custom")}
          </Button>
        </div>
      </div>
      {errorKey && <Alert message={translateError(locale, errorKey)} />}
      <div className="flex flex-wrap items-center gap-3">
        <div className="relative min-w-60 flex-1">
          <Search aria-hidden className="pointer-events-none absolute top-1/2 left-4 size-4 -translate-y-1/2 text-muted" />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t("teams.catalog.search")}
            aria-label={t("teams.catalog.search")}
            className="w-full rounded-full border border-line bg-raised py-2.5 pr-4 pl-10 text-ink outline-none placeholder:text-muted focus:border-accent"
          />
        </div>
        <SegmentedControl
          label={t("teams.catalog.categories")}
          value={category}
          onChange={setCategory}
          options={[
            { value: "all", label: t("teams.catalog.all") },
            ...rolesByGroup.map(({ group }) => ({ value: group, label: t(`teams.agent.roleGroups.${group}`) })),
          ]}
        />
      </div>
      {visibleTemplates.length === 0 && templates.length > 0 && <p className="py-6 text-center text-muted">{t("teams.catalog.empty")}</p>}
      <ul className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        {visibleTemplates.map((template) => (
          <li key={template.key} className="grid content-between gap-4 rounded-2xl border border-line bg-raised p-5">
            <div className="grid gap-3">
              <div className="flex items-center gap-3">
                <Avatar seed={groupOf(template)} label={template.name} className="size-12 rounded-xl text-xl" />
                <div className="min-w-0">
                  <h3 className="truncate font-bold text-ink">{template.name}</h3>
                  <p className="truncate text-sm text-muted">{template.role}</p>
                </div>
              </div>
              <p className="text-sm text-muted">{template.description}</p>
            </div>
            <div className="flex gap-3">
              <Button
                variant="ghost"
                size="pill"
                aria-label={`${t("teams.catalog.details")} ${template.name}`}
                onClick={() => open({ kind: "detail", template })}
              >
                {t("teams.catalog.details")}
              </Button>
              <Button
                size="pill"
                aria-label={`${t("teams.catalog.hire")} ${template.name}`}
                onClick={() => open({ kind: "hire", template })}
              >
                {t("teams.catalog.hire")}
              </Button>
            </div>
          </li>
        ))}
      </ul>
      {dialog?.kind === "detail" && (
        <Modal title={dialog.template.name} onClose={close}>
          <AgentTemplateDetail template={dialog.template} group={groupOf(dialog.template)} />
        </Modal>
      )}
      {dialog?.kind === "hire" && (
        <Modal title={`${t("teams.catalog.hire")} · ${dialog.template.name}`} onClose={close}>
          <HireWizard
            key={dialog.template.key}
            team={team}
            template={dialog.template}
            roleGroups={roleGroups}
            busy={busy}
            errorMessage={actionErrorMessage}
            onSubmit={create}
            onCancel={close}
          />
        </Modal>
      )}
      {dialog?.kind === "custom" && (
        <Modal title={`${t("teams.newAgent")} · ${team.name}`} onClose={close} bare>
          <AgentForm team={team} roleGroups={roleGroups} busy={busy} errorMessage={actionErrorMessage} onSubmit={create} onCancel={close} />
        </Modal>
      )}
    </div>
  );
}
