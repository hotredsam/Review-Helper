import {
  LayoutDashboard,
  BookOpen,
  ListChecks,
  GitBranch,
  Layers,
  MessageSquareQuote,
  MessagesSquare,
  Inbox,
  Palette,
  GraduationCap,
  Settings,
  type LucideIcon,
} from "lucide-react";

/** The navigable pane regions of the app. Each future phase fills in one of
 *  these; for now every region renders a clean empty state. */
export type SectionId =
  | "overview"
  | "understand"
  | "plan"
  | "decisions"
  | "stack"
  | "grill"
  | "chat"
  | "inbox"
  | "palette"
  | "learn"
  | "settings";

export interface Section {
  id: SectionId;
  label: string;
  icon: LucideIcon;
  emptyTitle: string;
  emptyBody: string;
}

// Ordered to follow the actual planning loop: understand the project, grill out
// scope, talk it through, capture ideas, choose the stack, record decisions —
// then the plan (the synthesis) and the assessment overview. New users land on
// Understand (the real entry point); returning users land where they left off.
export const SECTIONS: Section[] = [
  {
    id: "overview",
    label: "Overview",
    icon: LayoutDashboard,
    emptyTitle: "No assessment yet",
    emptyBody:
      "Once a model is connected and your repo is analyzed, this project's state and scores show up here.",
  },
  {
    id: "understand",
    label: "Understand",
    icon: BookOpen,
    emptyTitle: "Nothing to understand yet",
    emptyBody:
      "The Understand hub fills with concept cards as you work — architecture, frontend, backend, and more.",
  },
  {
    id: "grill",
    label: "Grill",
    icon: MessageSquareQuote,
    emptyTitle: "Not grilled yet",
    emptyBody:
      "Repo-specific questions that pin down what you're building show up here when you start grilling.",
  },
  {
    id: "chat",
    label: "Chat",
    icon: MessagesSquare,
    emptyTitle: "No conversation yet",
    emptyBody:
      "Talk through your project with the model here; proposals it makes become pending suggestions.",
  },
  {
    id: "inbox",
    label: "Inbox",
    icon: Inbox,
    emptyTitle: "Inbox empty",
    emptyBody: "Capture feature ideas as they come, then triage them into the plan later.",
  },
  {
    id: "stack",
    label: "Stack",
    icon: Layers,
    emptyTitle: "No stack chosen",
    emptyBody: "Frontend, backend, database, deployment and pipes choices appear here once selected.",
  },
  {
    id: "decisions",
    label: "Decisions",
    icon: GitBranch,
    emptyTitle: "No decisions recorded",
    emptyBody:
      "Decisions you make — and the ones the model proposes — collect here as a record you can revisit.",
  },
  {
    id: "plan",
    label: "Plan",
    icon: ListChecks,
    emptyTitle: "No plan yet",
    emptyBody: "Your phased build plan and its tasks will live here once the project is set up.",
  },
  {
    id: "palette",
    label: "Palette",
    icon: Palette,
    emptyTitle: "Color palette planner",
    emptyBody:
      "Design a frontend color theme and see it rendered as a generative app icon and a UI mock.",
  },
  {
    id: "learn",
    label: "Learn",
    icon: GraduationCap,
    emptyTitle: "Learning mode — coming soon",
    emptyBody: "A future mode for structured study beyond vibecoding (e.g. CPA exam prep). Not available yet.",
  },
  {
    id: "settings",
    label: "Settings",
    icon: Settings,
    emptyTitle: "Settings",
    emptyBody: "Model provider, themes and project options are configured here.",
  },
];

export const DEFAULT_SECTION: SectionId = "understand";

export function sectionById(id: SectionId): Section {
  return SECTIONS.find((s) => s.id === id) ?? SECTIONS[0];
}
