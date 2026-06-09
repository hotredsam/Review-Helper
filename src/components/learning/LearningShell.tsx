import { useEffect, useState } from "react";
import { GraduationCap, Plus, FileText, PencilLine } from "lucide-react";
import { EmptyState } from "../EmptyState";
import { useLearningStore } from "../../store/learningStore";
import { NewSubjectDialog } from "./NewSubjectDialog";
import { SubjectDetail } from "./SubjectDetail";

const STAGE_LABEL: Record<string, string> = {
  intake: "Scoping",
  proposed: "Choosing modules",
  ready: "Studying",
};

/** Learning-mode home: the subject library. Pick a subject to study it, or
 *  create one (describe a goal or upload material). When a subject is selected,
 *  its detail view takes over. Mirrors the project shell, for study instead. */
export function LearningShell() {
  const subjects = useLearningStore((s) => s.subjects);
  const status = useLearningStore((s) => s.status);
  const error = useLearningStore((s) => s.error);
  const load = useLearningStore((s) => s.load);
  const selectedId = useLearningStore((s) => s.selectedSubjectId);
  const select = useLearningStore((s) => s.select);
  const [dialogOpen, setDialogOpen] = useState(false);

  useEffect(() => {
    void load();
  }, [load]);

  if (selectedId != null) {
    return <SubjectDetail subjectId={selectedId} onBack={() => select(null)} />;
  }

  return (
    <div className="mx-auto max-w-4xl space-y-5 p-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-lg font-semibold text-fg">Learning</h1>
          <p className="text-sm text-fg-muted">
            Study anything — it grills you first, proposes a plan, then adapts to how you actually learn.
          </p>
        </div>
        <button
          onClick={() => setDialogOpen(true)}
          className="flex items-center gap-1.5 rounded-lg bg-accent px-3 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover"
        >
          <Plus className="h-4 w-4" />
          New subject
        </button>
      </div>

      {error && (
        <p className="text-sm text-danger" role="alert">
          {error}
        </p>
      )}

      {status === "loading" && subjects.length === 0 && (
        <p className="text-sm text-fg-subtle">Loading subjects…</p>
      )}

      {status === "ready" && subjects.length === 0 ? (
        <EmptyState
          icon={GraduationCap}
          title="No subjects yet"
          body="Add something you want to learn — describe a goal or upload material (a syllabus, notes, a PDF). It'll grill you on scope, then build a study plan that adapts to you."
          action={
            <button
              onClick={() => setDialogOpen(true)}
              className="rounded-lg bg-accent px-4 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover"
            >
              New subject
            </button>
          }
        />
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {subjects.map((s) => (
            <button
              key={s.id}
              onClick={() => select(s.id)}
              className="flex flex-col gap-2 rounded-xl border border-border bg-surface p-4 text-left transition-colors hover:border-accent hover:bg-surface-2"
            >
              <div className="flex items-center justify-between gap-2">
                <span className="font-medium text-fg">{s.title}</span>
                <span className="rounded-full bg-surface-2 px-2 py-0.5 text-xs text-fg-muted">
                  {STAGE_LABEL[s.stage] ?? s.stage}
                </span>
              </div>
              <span className="flex items-center gap-1.5 text-xs text-fg-subtle">
                {s.source_kind === "upload" ? <FileText className="h-3 w-3" /> : <PencilLine className="h-3 w-3" />}
                {s.source_kind === "upload" ? "From upload" : "Described"}
              </span>
            </button>
          ))}
        </div>
      )}

      <NewSubjectDialog open={dialogOpen} onClose={() => setDialogOpen(false)} />
    </div>
  );
}
