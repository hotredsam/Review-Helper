import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  body: string;
  action?: ReactNode;
}

/** Reusable, token-styled empty state for any not-yet-populated pane. */
export function EmptyState({ icon: Icon, title, body, action }: EmptyStateProps) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-4 p-10 text-center">
      <div className="flex h-14 w-14 items-center justify-center rounded-2xl border border-border bg-surface text-fg-subtle">
        <Icon className="h-7 w-7" strokeWidth={1.75} />
      </div>
      <div className="max-w-sm space-y-1.5">
        <h2 className="text-lg font-semibold text-fg">{title}</h2>
        <p className="text-sm text-fg-muted">{body}</p>
      </div>
      {action}
    </div>
  );
}
