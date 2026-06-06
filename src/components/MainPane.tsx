import { FolderPlus } from "lucide-react";
import { EmptyState } from "./EmptyState";
import { SettingsView } from "./SettingsView";
import { RepoCache } from "./RepoCache";
import { PlanPane } from "./PlanPane";
import { StatePane } from "./StatePane";
import { UnderstandHub } from "./UnderstandHub";
import { GrillPane } from "./GrillPane";
import { sectionById } from "../nav/sections";
import { useUiStore } from "../store/uiStore";
import { useProjectStore } from "../store/projectStore";

interface Props {
  onNewProject: () => void;
}

/** Main content area: the first-run prompt when no project exists, otherwise the
 *  active section (a clean empty state, or the Settings pane). */
export function MainPane({ onNewProject }: Props) {
  const activeSectionId = useUiStore((s) => s.activeSection);
  const projects = useProjectStore((s) => s.projects);
  const activeId = useProjectStore((s) => s.activeProjectId);
  const active = projects.find((p) => p.id === activeId) ?? null;
  const section = sectionById(activeSectionId);

  // No project: only Settings is reachable; everything else invites creating one.
  if (!active && section.id !== "settings") {
    return (
      <div className="flex h-full flex-col">
        <EmptyState
          icon={FolderPlus}
          title="Create your first project"
          body="Review Helper helps you plan and understand what you're building. Start with a project — a new build or an imported repo."
          action={
            <button
              onClick={onNewProject}
              className="rounded-lg bg-accent px-4 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover"
            >
              New project
            </button>
          }
        />
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center justify-between gap-3 border-b border-border px-6 py-4">
        <div>
          <p className="text-xs uppercase tracking-wide text-fg-subtle">
            {active ? active.name : "Review Helper"}
          </p>
          <h1 className="text-lg font-semibold text-fg">{section.label}</h1>
        </div>
        {active?.github_repo_url && <RepoCache project={active} />}
      </header>
      <div className="flex-1 overflow-auto">
        {section.id === "settings" ? (
          <SettingsView />
        ) : section.id === "overview" && active ? (
          <StatePane project={active} />
        ) : section.id === "understand" && active ? (
          <UnderstandHub />
        ) : section.id === "grill" && active ? (
          <GrillPane project={active} />
        ) : section.id === "plan" && active ? (
          <PlanPane project={active} />
        ) : (
          <EmptyState icon={section.icon} title={section.emptyTitle} body={section.emptyBody} />
        )}
      </div>
    </div>
  );
}
