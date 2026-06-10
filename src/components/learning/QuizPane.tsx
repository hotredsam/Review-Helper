import { useEffect, useState } from "react";
import { Check, Loader2, RotateCcw, X } from "lucide-react";
import { modelStop } from "../../api/model";
import { type QuizQuestion, type QuizResult, learningQuiz, learningQuizAnswer } from "../../api/learning";

/** Multiple-choice retrieval practice. Answering records the attempt, updates the
 *  skill's mastery, and reveals the correct answer + explanation immediately —
 *  active recall is the best-evidenced way to learn. */
export function QuizPane({ moduleId, onAnswered }: { moduleId: number; onAnswered?: () => void }) {
  const [qs, setQs] = useState<QuizQuestion[] | null>(null);
  const [i, setI] = useState(0);
  const [picked, setPicked] = useState<number | null>(null);
  const [result, setResult] = useState<QuizResult | null>(null);
  const [score, setScore] = useState(0);
  const [startedAt, setStartedAt] = useState(0);
  const [done, setDone] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    setQs(null);
    setI(0);
    setPicked(null);
    setResult(null);
    setScore(0);
    setDone(false);
    setError(null);
    learningQuiz(moduleId)
      .then((q) => {
        if (!live) return;
        setQs(q);
        setStartedAt(Date.now());
      })
      .catch((e) => live && setError(String(e)));
    return () => {
      live = false;
    };
  }, [moduleId]);

  if (error) {
    return (
      <p className="text-sm text-danger" role="alert">
        {error}
      </p>
    );
  }
  if (qs === null) {
    return (
      <p className="flex items-center gap-2 text-sm text-fg-subtle">
        <Loader2 className="h-4 w-4 animate-spin" /> Writing your quiz…
        <button
          type="button"
          onClick={() => void modelStop(`learning:${moduleId}`)}
          className="rounded-md border border-border px-2 py-0.5 text-xs text-fg-muted hover:bg-surface-2"
        >
          Cancel
        </button>
      </p>
    );
  }
  if (qs.length === 0) {
    return <p className="text-sm text-fg-subtle">No questions in this module.</p>;
  }

  if (done) {
    return (
      <div className="rounded-xl border border-border bg-surface p-8 text-center">
        <p className="text-lg font-semibold text-fg">
          {score} / {qs.length} correct
        </p>
        <p className="mt-1 text-sm text-fg-muted">Your mastery estimate updated from every answer — see Progress.</p>
        <button
          onClick={() => {
            setI(0);
            setPicked(null);
            setResult(null);
            setScore(0);
            setDone(false);
            setStartedAt(Date.now());
          }}
          className="mt-4 inline-flex items-center gap-1.5 rounded-lg border border-border px-3 py-1.5 text-sm text-fg-muted hover:bg-surface-2"
        >
          <RotateCcw className="h-4 w-4" />
          Retake
        </button>
      </div>
    );
  }

  const q = qs[i];

  const answer = async (idx: number) => {
    if (result) return;
    setPicked(idx);
    try {
      const r = await learningQuizAnswer(q.id, idx, Date.now() - startedAt);
      setResult(r);
      if (r.correct) setScore((s) => s + 1);
      onAnswered?.();
    } catch (e) {
      setError(String(e));
      setPicked(null);
    }
  };

  const next = () => {
    setPicked(null);
    setResult(null);
    setStartedAt(Date.now());
    if (i + 1 < qs.length) setI(i + 1);
    else setDone(true);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between text-xs text-fg-subtle">
        <span>
          Question {i + 1} of {qs.length}
        </span>
        {(i > 0 || result) && (
          <span>
            Score {score}/{qs.length}
          </span>
        )}
      </div>

      <p className="text-base font-medium text-fg">{q.question}</p>

      <div className="space-y-2">
        {q.options.map((opt, idx) => {
          const isAnswer = result && idx === result.answer_idx;
          const isWrongPick = result && idx === picked && !result.correct;
          let cls = "border-border hover:bg-surface-2";
          if (isAnswer) cls = "border-success bg-success/10 text-fg";
          else if (isWrongPick) cls = "border-danger bg-danger/10 text-fg";
          else if (result) cls = "border-border opacity-60";
          return (
            <button
              key={idx}
              onClick={() => void answer(idx)}
              disabled={!!result}
              className={"flex w-full items-center justify-between gap-2 rounded-lg border px-4 py-2.5 text-left text-sm transition-colors disabled:cursor-default " + cls}
            >
              <span>{opt}</span>
              {isAnswer && <Check className="h-4 w-4 text-success" />}
              {isWrongPick && <X className="h-4 w-4 text-danger" />}
            </button>
          );
        })}
      </div>

      {result && (
        <div className="rounded-lg border border-border bg-surface-2 p-3">
          <p className={"text-sm font-medium " + (result.correct ? "text-success" : "text-danger")}>
            {result.correct ? "Correct" : "Not quite"}
          </p>
          {result.explanation && <p className="mt-1 text-sm text-fg-muted">{result.explanation}</p>}
          <button
            onClick={next}
            className="mt-3 rounded-lg bg-accent px-3 py-1.5 text-sm font-medium text-accent-fg hover:bg-accent-hover"
          >
            {i + 1 < qs.length ? "Next question" : "See score"}
          </button>
        </div>
      )}
    </div>
  );
}
