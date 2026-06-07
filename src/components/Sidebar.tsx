import { Menu } from "lucide-react";
import { SECTIONS } from "../nav/sections";
import { useUiStore } from "../store/uiStore";
import { ProjectSwitcher } from "./ProjectSwitcher";

interface Props {
  onNewProject: () => void;
  hasProject: boolean;
}

/** Left rail: hamburger toggle, project switcher, and the section nav. Sections
 *  other than Settings and the Learn stub are disabled until a project exists. */
export function Sidebar({ onNewProject, hasProject }: Props) {
  const collapsed = useUiStore((s) => s.sidebarCollapsed);
  const toggle = useUiStore((s) => s.toggleSidebar);
  const active = useUiStore((s) => s.activeSection);
  const setSection = useUiStore((s) => s.setSection);

  return (
    <aside
      className={
        "flex h-full shrink-0 flex-col gap-3 border-r border-border bg-surface p-3 transition-all duration-200 " +
        (collapsed ? "w-16" : "w-60")
      }
    >
      <div className="flex items-center gap-2">
        <button
          onClick={toggle}
          title="Toggle sidebar"
          aria-label="Toggle sidebar"
          className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg text-fg-muted hover:bg-surface-2 hover:text-fg"
        >
          <Menu className="h-5 w-5" />
        </button>
        {!collapsed && (
          <span className="truncate text-sm font-semibold text-fg">Review Helper</span>
        )}
      </div>

      <ProjectSwitcher collapsed={collapsed} onNewProject={onNewProject} />

      <nav className="flex flex-1 flex-col gap-0.5 overflow-y-auto">
        {SECTIONS.map((s) => {
          const Icon = s.icon;
          const isActive = s.id === active;
          const disabled = !hasProject && s.id !== "settings" && s.id !== "learn";
          return (
            <button
              key={s.id}
              onClick={() => setSection(s.id)}
              disabled={disabled}
              aria-label={s.label}
              aria-current={isActive ? "page" : undefined}
              title={collapsed ? s.label : undefined}
              className={
                "flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-ring/40 " +
                (collapsed ? "justify-center " : "") +
                (isActive
                  ? "bg-accent/10 font-medium text-fg"
                  : "text-fg-muted hover:bg-surface-2 hover:text-fg") +
                (disabled ? " cursor-not-allowed opacity-40 hover:bg-transparent hover:text-fg-muted" : "")
              }
            >
              <Icon className="h-5 w-5 shrink-0" strokeWidth={1.75} />
              {!collapsed && <span className="truncate">{s.label}</span>}
            </button>
          );
        })}
      </nav>
    </aside>
  );
}
