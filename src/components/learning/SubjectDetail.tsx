import { useEffect, useState } from "react";
import { ArrowLeft, Trash2 } from "lucide-react";
import { type SubjectDetail as SubjectDetailData, subjectGet } from "../../api/learning";
import { useLearningStore } from "../../store/learningStore";

const STAGE_LABEL: Record<string, string> = {
  intake: "Scoping",
  proposed: "Choosing modules",
  ready: "Studying",
};

/** A single subject's workspace. L0 shows the overview; later sub-phases add the
 *  intake grill, module proposal, and the generated study materials as tabs. */
export function SubjectDetail({ subjectId, onBack }: { subjectId: number; onBack: () => void }) {
  const remove = useLearningStore((s) => s.remove);
  const [detail, setDetail] = useState<SubjectDetailData | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setDetail(null);
    setError(null);
    subjectGet(subjectId)
      .then(setDetail)
      .catch((e) => setError(String(e)));
  }, [subjectId]);

  const onDelete = async () => {
    if (!confirm("Delete this subject and all its study materials? This can't be undone.")) return;
    try {
      await remove(subjectId);
      onBack();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="mx-auto max-w-4xl space-y-5 p-8">
      <div className="flex items-center justify-between gap-3">
        <button
          onClick={onBack}
          className="flex items-center gap-1.5 text-sm text-fg-muted hover:text-fg"
        >
          <ArrowLeft className="h-4 w-4" />
          All subjects
        </button>
        <button
          onClick={() => void onDelete()}
          aria-label="Delete subject"
          className="flex items-center gap-1.5 rounded-lg border border-border px-2.5 py-1 text-xs text-fg-muted hover:bg-surface-2 hover:text-danger"
        >
          <Trash2 className="h-3.5 w-3.5" />
          Delete
        </button>
      </div>

      {error && (
        <p className="text-sm text-danger" role="alert">
          {error}
        </p>
      )}

      {!detail && !error && <p className="text-sm text-fg-subtle">Loading…</p>}

      {detail && (
        <>
          <div className="flex items-center gap-3">
            <h1 className="text-xl font-semibold text-fg">{detail.title}</h1>
            <span className="rounded-full bg-surface-2 px-2 py-0.5 text-xs text-fg-muted">
              {STAGE_LABEL[detail.stage] ?? detail.stage}
            </span>
          </div>

          <section className="rounded-xl border border-border bg-surface p-5">
            <h2 className="mb-2 text-xs font-semibold uppercase tracking-wide text-fg-subtle">
              {detail.source_kind === "upload" ? "From your upload" : "What you want to learn"}
            </h2>
            <p className="whitespace-pre-wrap text-sm text-fg-muted">
              {detail.source_text?.trim() || "No description yet."}
            </p>
          </section>

          <p className="text-sm text-fg-subtle">
            Next, this subject gets scoped with a few questions, then a study plan is proposed. (Building.)
          </p>
        </>
      )}
    </div>
  );
}
