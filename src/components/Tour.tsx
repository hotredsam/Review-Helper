import { useState } from "react";
import { X, ArrowRight, ArrowLeft } from "lucide-react";

export interface TourStep {
  title: string;
  body: string;
}

export const TOUR_STEPS: TourStep[] = [
  {
    title: "Welcome to Review Helper",
    body: "Plan and understand what you're building — the right way. Start with a project: a brand-new build, or an imported GitHub repo (connect GitHub, then pick, create, or link a repo).",
  },
  {
    title: "Understand & Plan",
    body: "The Understand hub explains any concept on demand — tech and product alike. The Plan pane turns your repo or description into an honest, phased plan.",
  },
  {
    title: "Grill & Chat",
    body: "Grill asks sharp, repo-specific questions (with recommended answers) to pin down scope. Chat talks it through — anything it infers becomes a pending suggestion you approve.",
  },
  {
    title: "Decisions, Stack & Inbox",
    body: "Approve suggestions into your decisions record, choose your stack from recommendations, and capture feature ideas in the inbox to weave into the plan later.",
  },
  {
    title: "Sync to GitHub",
    body: "Push your planning package and one issue per phase to GitHub — every change to main, including closes and deletions, is previewed before anything is written.",
  },
];

const SEEN_KEY = "rh.tour.seen";

export function tourSeen(): boolean {
  try {
    return localStorage.getItem(SEEN_KEY) === "1";
  } catch {
    return false;
  }
}
export function markTourSeen() {
  try {
    localStorage.setItem(SEEN_KEY, "1");
  } catch {
    /* ignore */
  }
}

/** A 5-step welcome tour. Fixed content (no runtime LLM UI). */
export function Tour({ onClose }: { onClose: () => void }) {
  const [i, setI] = useState(0);
  const step = TOUR_STEPS[i];
  const last = i === TOUR_STEPS.length - 1;
  const close = () => {
    markTourSeen();
    onClose();
  };
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"
      role="dialog"
      aria-modal="true"
      aria-label="Welcome tour"
    >
      <div className="w-full max-w-md rounded-xl border border-border bg-surface p-6 shadow-xl">
        <div className="mb-2 flex items-center justify-between">
          <span className="text-xs text-fg-subtle">
            Step {i + 1} of {TOUR_STEPS.length}
          </span>
          <button type="button" onClick={close} aria-label="Skip tour" className="text-fg-subtle hover:text-fg">
            <X className="h-4 w-4" />
          </button>
        </div>
        <h2 className="text-lg font-semibold text-fg">{step.title}</h2>
        <p className="mt-2 text-sm text-fg-muted">{step.body}</p>
        <div className="mt-5 flex items-center justify-between">
          <button
            type="button"
            onClick={() => setI((v) => Math.max(0, v - 1))}
            disabled={i === 0}
            className="flex items-center gap-1 text-sm text-fg-muted disabled:opacity-40"
          >
            <ArrowLeft className="h-4 w-4" /> Back
          </button>
          {last ? (
            <button
              type="button"
              onClick={close}
              className="rounded-lg bg-accent px-4 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover"
            >
              Get started
            </button>
          ) : (
            <button
              type="button"
              onClick={() => setI((v) => v + 1)}
              className="flex items-center gap-1 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover"
            >
              Next <ArrowRight className="h-4 w-4" />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
