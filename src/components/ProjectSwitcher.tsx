import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { ChevronsUpDown, Plus, Check, Folder } from "lucide-react";
import { useProjectStore } from "../store/projectStore";

interface Props {
  collapsed: boolean;
  onNewProject: () => void;
}

/** Active-project picker. Collapsed, it's a single "new project" affordance;
 *  expanded, a dropdown to switch between projects or create one. Fully keyboard
 *  operable: the trigger exposes listbox semantics, options are a roving-focus
 *  listbox, Arrow keys move between them, and Escape closes + restores focus. */
export function ProjectSwitcher({ collapsed, onNewProject }: Props) {
  const projects = useProjectStore((s) => s.projects);
  const activeId = useProjectStore((s) => s.activeProjectId);
  const select = useProjectStore((s) => s.select);
  const [open, setOpen] = useState(false);
  const active = projects.find((p) => p.id === activeId) ?? null;

  const triggerRef = useRef<HTMLButtonElement>(null);
  const optionRefs = useRef<(HTMLButtonElement | null)[]>([]);

  // On open, move focus to the active option (or the first one) so keyboard
  // users land inside the listbox; trim stale refs as the list changes.
  useEffect(() => {
    if (!open) return;
    optionRefs.current = optionRefs.current.slice(0, projects.length);
    const activeIdx = Math.max(
      0,
      projects.findIndex((p) => p.id === activeId),
    );
    optionRefs.current[activeIdx]?.focus();
  }, [open, projects, activeId]);

  const close = (restoreFocus = true) => {
    setOpen(false);
    if (restoreFocus) triggerRef.current?.focus();
  };

  const moveFocus = (delta: number) => {
    const items = optionRefs.current.filter(Boolean) as HTMLButtonElement[];
    if (items.length === 0) return;
    const current = items.indexOf(document.activeElement as HTMLButtonElement);
    const next = (current + delta + items.length) % items.length;
    items[next].focus();
  };

  const onMenuKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      moveFocus(1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      moveFocus(-1);
    }
  };

  const onTriggerKeyDown = (e: KeyboardEvent<HTMLButtonElement>) => {
    if (!open && (e.key === "ArrowDown" || e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      setOpen(true);
    }
  };

  if (collapsed) {
    return (
      <button
        onClick={onNewProject}
        title="New project"
        aria-label="New project"
        className="flex h-10 w-10 items-center justify-center rounded-lg border border-border text-fg-muted hover:bg-surface-2 hover:text-fg focus:outline-none focus:ring-2 focus:ring-ring/60"
      >
        <Folder className="h-5 w-5" />
      </button>
    );
  }

  return (
    <div className="relative">
      <button
        ref={triggerRef}
        onClick={() => setOpen((o) => !o)}
        onKeyDown={onTriggerKeyDown}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={active ? `Current project: ${active.name}. Switch project` : "Choose a project"}
        className="flex w-full items-center justify-between gap-2 rounded-lg border border-border bg-surface px-3 py-2 text-left text-sm hover:bg-surface-2 focus:outline-none focus:ring-2 focus:ring-ring/60"
      >
        <span className="truncate font-medium text-fg">
          {active ? active.name : "No project"}
        </span>
        <ChevronsUpDown className="h-4 w-4 shrink-0 text-fg-subtle" />
      </button>

      {open && (
        <>
          <div className="fixed inset-0 z-20" onClick={() => close(false)} role="presentation" />
          <div
            className="absolute z-30 mt-1 w-full overflow-hidden rounded-lg border border-border bg-overlay shadow-lg"
            onKeyDown={onMenuKeyDown}
          >
            {projects.length > 0 && (
              <ul className="max-h-60 overflow-auto py-1" role="listbox" aria-label="Projects">
                {projects.map((p, i) => (
                  <li key={p.id} role="presentation">
                    <button
                      ref={(el) => {
                        optionRefs.current[i] = el;
                      }}
                      role="option"
                      aria-selected={p.id === activeId}
                      onClick={() => {
                        select(p.id);
                        close();
                      }}
                      className="flex w-full items-center justify-between gap-2 px-3 py-2 text-left text-sm text-fg hover:bg-surface-2 focus:bg-surface-2 focus:outline-none focus:ring-2 focus:ring-inset focus:ring-ring/60"
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
                close(false);
                onNewProject();
              }}
              className="flex w-full items-center gap-2 border-t border-border px-3 py-2 text-left text-sm font-medium text-accent hover:bg-surface-2 focus:bg-surface-2 focus:outline-none focus:ring-2 focus:ring-inset focus:ring-ring/60"
            >
              <Plus className="h-4 w-4" /> New project
            </button>
          </div>
        </>
      )}
    </div>
  );
}
