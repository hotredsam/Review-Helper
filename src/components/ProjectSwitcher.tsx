import { useState } from "react";
import { ChevronsUpDown, Plus, Check, Folder } from "lucide-react";
import { useProjectStore } from "../store/projectStore";

interface Props {
  collapsed: boolean;
  onNewProject: () => void;
}

/** Active-project picker. Collapsed, it's a single "new project" affordance;
 *  expanded, a dropdown to switch between projects or create one. */
export function ProjectSwitcher({ collapsed, onNewProject }: Props) {
  const projects = useProjectStore((s) => s.projects);
  const activeId = useProjectStore((s) => s.activeProjectId);
  const select = useProjectStore((s) => s.select);
  const [open, setOpen] = useState(false);
  const active = projects.find((p) => p.id === activeId) ?? null;

  if (collapsed) {
    return (
      <button
        onClick={onNewProject}
        title="New project"
        aria-label="New project"
        className="flex h-10 w-10 items-center justify-center rounded-lg border border-border text-fg-muted hover:bg-surface-2 hover:text-fg"
      >
        <Folder className="h-5 w-5" />
      </button>
    );
  }

  return (
    <div className="relative">
      <button
        onClick={() => setOpen((o) => !o)}
        className="flex w-full items-center justify-between gap-2 rounded-lg border border-border bg-surface px-3 py-2 text-left text-sm hover:bg-surface-2"
      >
        <span className="truncate font-medium text-fg">
          {active ? active.name : "No project"}
        </span>
        <ChevronsUpDown className="h-4 w-4 shrink-0 text-fg-subtle" />
      </button>

      {open && (
        <>
          <div className="fixed inset-0 z-20" onClick={() => setOpen(false)} role="presentation" />
          <div className="absolute z-30 mt-1 w-full overflow-hidden rounded-lg border border-border bg-overlay shadow-lg">
            {projects.length > 0 && (
              <ul className="max-h-60 overflow-auto py-1">
                {projects.map((p) => (
                  <li key={p.id}>
                    <button
                      onClick={() => {
                        select(p.id);
                        setOpen(false);
                      }}
                      className="flex w-full items-center justify-between gap-2 px-3 py-2 text-left text-sm text-fg hover:bg-surface-2"
                    >
                      <span className="truncate">{p.name}</span>
                      {p.id === activeId && <Check className="h-4 w-4 shrink-0 text-accent" />}
                    </button>
                  </li>
                ))}
              </ul>
            )}
            <button
              onClick={() => {
                setOpen(false);
                onNewProject();
              }}
              className="flex w-full items-center gap-2 border-t border-border px-3 py-2 text-left text-sm font-medium text-accent hover:bg-surface-2"
            >
              <Plus className="h-4 w-4" /> New project
            </button>
          </div>
        </>
      )}
    </div>
  );
}
