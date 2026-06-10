import { useEffect, useState } from "react";
import { Loader2, RotateCcw } from "lucide-react";
import { modelStop } from "../../api/model";
import { type Flashcard, learningFlashcards, learningFlashcardGrade } from "../../api/learning";

// FSRS grades. Again/Hard count as lapses; Good/Easy as recalled (drives mastery).
const GRADES: { rating: 1 | 2 | 3 | 4; label: string; cls: string }[] = [
  { rating: 1, label: "Again", cls: "border-danger/40 text-danger hover:bg-danger/10" },
  { rating: 2, label: "Hard", cls: "border-border text-fg-muted hover:bg-surface-2" },
  { rating: 3, label: "Good", cls: "border-border text-fg hover:bg-surface-2" },
  { rating: 4, label: "Easy", cls: "border-success/40 text-success hover:bg-success/10" },
];

/** Spaced-repetition flashcards. Flip to reveal, then grade — the grade feeds the
 *  FSRS scheduler (when each card resurfaces) and the skill's mastery estimate. */
export function FlashcardsPane({ moduleId }: { moduleId: number }) {
  const [cards, setCards] = useState<Flashcard[] | null>(null);
  const [i, setI] = useState(0);
  const [flipped, setFlipped] = useState(false);
  const [done, setDone] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    setCards(null);
    setI(0);
    setFlipped(false);
    setDone(false);
    setError(null);
    learningFlashcards(moduleId)
      .then((c) => live && setCards(c))
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
  if (cards === null) {
    return (
      <p className="flex items-center gap-2 text-sm text-fg-subtle">
        <Loader2 className="h-4 w-4 animate-spin" /> Building your flashcards…
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
  if (cards.length === 0) {
    return <p className="text-sm text-fg-subtle">No flashcards in this module.</p>;
  }

  const restart = () => {
    setI(0);
    setFlipped(false);
    setDone(false);
  };

  if (done) {
    return (
      <div className="rounded-xl border border-border bg-surface p-8 text-center">
        <p className="text-sm text-fg">Reviewed all {cards.length} cards. They'll resurface when they're due.</p>
        <button
          onClick={restart}
          className="mt-4 inline-flex items-center gap-1.5 rounded-lg border border-border px-3 py-1.5 text-sm text-fg-muted hover:bg-surface-2"
        >
          <RotateCcw className="h-4 w-4" />
          Review again
        </button>
      </div>
    );
  }

  const card = cards[i];

  const grade = async (rating: 1 | 2 | 3 | 4) => {
    try {
      await learningFlashcardGrade(card.id, rating);
    } catch (e) {
      setError(String(e));
      return;
    }
    if (i + 1 < cards.length) {
      setI(i + 1);
      setFlipped(false);
    } else {
      setDone(true);
    }
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between text-xs text-fg-subtle">
        <span>
          Card {i + 1} of {cards.length}
        </span>
        {error && (
          <span className="text-danger" role="alert">
            {error}
          </span>
        )}
      </div>

      <button
        type="button"
        onClick={() => setFlipped((f) => !f)}
        className="flex min-h-[180px] w-full flex-col items-center justify-center gap-2 rounded-2xl border border-border bg-surface p-8 text-center transition-colors hover:border-accent"
      >
        <span className="text-lg font-medium text-fg">{card.front}</span>
        {flipped ? (
          <span className="mt-2 border-t border-border pt-3 text-sm text-fg-muted">{card.back}</span>
        ) : (
          <span className="mt-2 text-xs text-fg-subtle">Click to reveal</span>
        )}
      </button>

      {flipped ? (
        <div className="grid grid-cols-4 gap-2">
          {GRADES.map((g) => (
            <button
              key={g.rating}
              onClick={() => void grade(g.rating)}
              className={"rounded-lg border px-3 py-2 text-sm font-medium transition-colors " + g.cls}
            >
              {g.label}
            </button>
          ))}
        </div>
      ) : (
        <p className="text-center text-xs text-fg-subtle">Reveal the answer, then rate how well you knew it.</p>
      )}
    </div>
  );
}
