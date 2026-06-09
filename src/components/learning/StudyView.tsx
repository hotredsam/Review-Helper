import { useEffect, useState } from "react";
import { FileText, Layers, ListChecks, Loader2, MessageCircle, TrendingUp, type LucideIcon } from "lucide-react";
import { type ModuleKind, type ProposedModule, learningModules } from "../../api/learning";
import { NotesPane } from "./NotesPane";
import { FlashcardsPane } from "./FlashcardsPane";
import { QuizPane } from "./QuizPane";
import { ProgressPane } from "./ProgressPane";
import { TutorPane } from "./TutorPane";

const KIND_ICON: Record<ModuleKind, LucideIcon> = {
  notes: FileText,
  flashcards: Layers,
  quiz: ListChecks,
  tutor: TrendingUp,
};

/** The "ready" study workspace: a tab per included module (notes/flashcards/
 *  quiz) plus a Progress tab. Materials generate on first open; Progress reflects
 *  the adaptive learner profile, refreshed each time it's opened. */
export function StudyView({ subjectId }: { subjectId: number }) {
  const [modules, setModules] = useState<ProposedModule[] | null>(null);
  const [tab, setTab] = useState<string>("progress");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    learningModules(subjectId)
      .then((m) => {
        if (!live) return;
        const inc = m.filter((x) => x.included);
        setModules(inc);
        setTab(inc[0] ? `m${inc[0].id}` : "progress");
      })
      .catch((e) => live && setError(String(e)));
    return () => {
      live = false;
    };
  }, [subjectId]);

  if (error) {
    return (
      <p className="text-sm text-danger" role="alert">
        {error}
      </p>
    );
  }
  if (!modules) {
    return (
      <p className="flex items-center gap-2 text-sm text-fg-subtle">
        <Loader2 className="h-4 w-4 animate-spin" /> Loading your study plan…
      </p>
    );
  }

  const active = modules.find((m) => `m${m.id}` === tab) ?? null;

  const TabButton = ({ id, label, icon: Icon }: { id: string; label: string; icon: LucideIcon }) => {
    const on = tab === id;
    return (
      <button
        onClick={() => setTab(id)}
        aria-current={on ? "page" : undefined}
        className={
          "flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-sm transition-colors " +
          (on ? "bg-accent/10 font-medium text-fg" : "text-fg-muted hover:bg-surface-2 hover:text-fg")
        }
      >
        <Icon className="h-4 w-4" />
        {label}
      </button>
    );
  };

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap gap-1 border-b border-border pb-2">
        {modules.map((m) => (
          <TabButton key={m.id} id={`m${m.id}`} label={m.title} icon={KIND_ICON[m.kind] ?? FileText} />
        ))}
        <TabButton id="tutor" label="Tutor" icon={MessageCircle} />
        <TabButton id="progress" label="Progress" icon={TrendingUp} />
      </div>

      <div>
        {active?.kind === "notes" && <NotesPane moduleId={active.id} />}
        {active?.kind === "flashcards" && <FlashcardsPane moduleId={active.id} />}
        {active?.kind === "quiz" && <QuizPane moduleId={active.id} />}
        {tab === "tutor" && <TutorPane subjectId={subjectId} />}
        {tab === "progress" && <ProgressPane subjectId={subjectId} />}
      </div>
    </div>
  );
}
