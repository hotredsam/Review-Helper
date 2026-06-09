import { useEffect, useState } from "react";
import { Loader2, MessageSquareQuote } from "lucide-react";
import {
  type IntakeItem,
  type SubjectDetail,
  learningIntake,
  learningIntakeAnswer,
} from "../../api/learning";

/**
 * L1 — the intake grill. Before any material is built, the subject is scoped with
 * a few sharp questions (level, goal, time, depth, how it'll be used). Answers
 * are saved as you go and later drive the module proposal. Never dead-ends: the
 * generate/save failure paths surface inline.
 */
export function IntakePane({
  subject,
  onReadyToPropose,
}: {
  subject: SubjectDetail;
  onReadyToPropose: (answered: number, total: number) => void;
}) {
  const [items, setItems] = useState<IntakeItem[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load already-generated questions (cheap; no model call if cached).
  useEffect(() => {
    let live = true;
    setItems(null);
    setError(null);
    setBusy(true);
    learningIntake(subject.id)
      .then((qs) => {
        if (!live) return;
        setItems(qs);
        onReadyToPropose(qs.filter((q) => q.answer?.trim()).length, qs.length);
      })
      .catch((e) => live && setError(String(e)))
      .finally(() => live && setBusy(false));
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [subject.id]);

  const saveAnswer = async (item: IntakeItem, value: string) => {
    setItems((prev) => prev?.map((q) => (q.id === item.id ? { ...q, answer: value } : q)) ?? prev);
    try {
      await learningIntakeAnswer(item.id, value);
      setItems((prev) => {
        const next = prev?.map((q) => (q.id === item.id ? { ...q, answer: value } : q)) ?? prev;
        if (next) onReadyToPropose(next.filter((q) => q.answer?.trim()).length, next.length);
        return next;
      });
    } catch (e) {
      setError(String(e));
    }
  };

  if (error && !items) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-danger" role="alert">
          {error}
        </p>
        <p className="text-sm text-fg-subtle">
          Scoping needs the model. Check it's connected (Settings) and try reopening this subject.
        </p>
      </div>
    );
  }

  if (busy && !items) {
    return (
      <p className="flex items-center gap-2 text-sm text-fg-subtle">
        <Loader2 className="h-4 w-4 animate-spin" /> Scoping this subject — a few quick questions…
      </p>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2 text-sm text-fg-muted">
        <MessageSquareQuote className="h-4 w-4 text-accent" />
        Answer what you can — this tailors your study plan. Skip anything that doesn't apply.
      </div>
      {error && (
        <p className="text-sm text-danger" role="alert">
          {error}
        </p>
      )}
      <ol className="space-y-3">
        {items?.map((item) => (
          <li key={item.id} className="rounded-xl border border-border bg-surface p-4">
            <label className="block text-sm font-medium text-fg" htmlFor={`intake-${item.id}`}>
              {item.question}
            </label>
            <textarea
              id={`intake-${item.id}`}
              defaultValue={item.answer ?? ""}
              onBlur={(e) => void saveAnswer(item, e.target.value)}
              rows={2}
              maxLength={4000}
              placeholder="Your answer…"
              className="mt-2 w-full resize-y rounded-lg border border-border bg-bg px-3 py-2 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none focus:ring-2 focus:ring-ring/40"
            />
          </li>
        ))}
      </ol>
    </div>
  );
}
