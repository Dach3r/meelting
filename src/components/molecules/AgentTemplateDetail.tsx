import type { AgentTemplate } from "../../hooks/useAgentTemplates";
import { Avatar } from "../atoms/Avatar";

export function AgentTemplateDetail({ template }: { template: AgentTemplate }) {
  const [lead, ...sections] = template.instructions.split("\n\n");
  return (
    <div className="grid gap-5 px-6 pt-4 pb-6">
      <div className="flex items-center gap-4">
        <Avatar seed={template.key} label={template.name} className="size-16 rounded-xl text-2xl" />
        <div>
          <p className="font-bold text-ink">{template.name}</p>
          <p className="text-sm text-muted">{template.role}</p>
        </div>
      </div>
      <p className="text-ink">{template.description}</p>
      <div className="grid gap-4 text-sm text-ink">
        <p>{lead}</p>
        {sections.map((section) => {
          const [heading, ...lines] = section.split("\n");
          return (
            <div key={heading} className="grid gap-1.5">
              <h3 className="font-bold">{heading.replace(/:$/, "")}</h3>
              <ul className="grid list-disc gap-1 pl-5 text-muted">
                {lines.map((line) => (
                  <li key={line}>{line.replace(/^- /, "")}</li>
                ))}
              </ul>
            </div>
          );
        })}
      </div>
    </div>
  );
}
