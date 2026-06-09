import { useCallback, useEffect, useState } from "react";
import { ArrowLeft, Loader2, Sparkles, Trash2 } from "lucide-react";
import { type SubjectDetail as SubjectDetailData, subjectGet, learningPropose } from "../../api/learning";
import { useLearningStore } from "../../store/learningStore";
import { IntakePane } from "./IntakePane";
import { ModuleProposalPane } from "./ModuleProposalPane";
import { StudyView } from "./StudyView";

const STAGE_LABEL: Record<string, string> = {
  intake: "Scoping",
  proposed: "Choosing modules",
  ready: "Studying",
};

/** A single subject's workspace, routed by stage: scope it (intake grill), pick
 *  a study plan (module proposal), then study the generated materials. */
export function SubjectDetail({ subjectId, onBack }: { subjectId: number; onBack: () => void }) {
  const remove = useLearningStore((s) => s.remove);
  const reloadList = useLearningStore((s) => s.load);
  const [detail, setDetail] = useState<SubjectDetailData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [answered, setAnswered] = useState(0);
  const [proposing, setProposing] = useState(false);

  const load = useCallback(() => {
    return subjectGet(subjectId)
      .then(setDetail)
      .catch((e) => setError(String(e)));
  }, [subjectId]);

  useEffect(() => {
    setDetail(null);
    setError(null);
    setAnswered(0);
    void load();
  }, [load]);

  // Refetch the subject (e.g. after its stage advances) + refresh the list badges.
  const reload = useCallback(async () => {
    await load();
    await reloadList();
  }, [load, reloadList]);

  const propose = async () => {
    setProposing(true);
    setError(null);
    try {
      await learningPropose(subjectId);
      await reload();
    } catch (e) {
      setError(String(e));
    } finally {
      setProposing(false);
    }
  };

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
        <button onClick={onBack} className="flex items-center gap-1.5 text-sm text-fg-muted hover:text-fg">
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
          <div className="space-y-1">
            <div className="flex items-center gap-3">
              <h1 className="text-xl font-semibold text-fg">{detail.title}</h1>
              <span className="rounded-full bg-surface-2 px-2 py-0.5 text-xs text-fg-muted">
                {STAGE_LABEL[detail.stage] ?? detail.stage}
              </span>
            </div>
            {detail.source_text?.trim() && (
              <p className="line-clamp-3 text-sm text-fg-muted">{detail.source_text}</p>
            )}
          </div>

          {detail.stage === "intake" && (
            <>
              <IntakePane subject={detail} onReadyToPropose={setAnswered} />
              <div className="flex items-center justify-between gap-3 border-t border-border pt-4">
                <span className="text-xs text-fg-subtle">
                  {answered === 0 ? "Answer a few to tailor the plan." : `${answered} answered`}
                </span>
                <button
                  onClick={() => void propose()}
                  disabled={proposing || answered === 0}
                  title={answered === 0 ? "Answer at least one question first" : undefined}
                  className="flex items-center gap-1.5 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
                >
                  {proposing ? <Loader2 className="h-4 w-4 animate-spin" /> : <Sparkles className="h-4 w-4" />}
                  {proposing ? "Designing your plan…" : "Propose study plan"}
                </button>
              </div>
            </>
          )}

          {detail.stage === "proposed" && <ModuleProposalPane subjectId={subjectId} onConfirmed={() => void reload()} />}

          {detail.stage === "ready" && <StudyView subjectId={subjectId} />}
        </>
      )}
    </div>
  );
}
