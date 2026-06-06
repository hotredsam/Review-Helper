import { useState } from "react";
import { Loader2, HelpCircle } from "lucide-react";
import { cardExplain, type Card } from "../api/cards";

/**
 * A "Why?" affordance for a decision or stack choice: surfaces an explanation
 * (generating + caching a learning card for the term) inline. The card is then
 * retrievable from the Understand hub — never a dead end.
 */
export function WhyExplain({ term }: { term: string }) {
  const [card, setCard] = useState<Card | null>(null);
  const [busy, setBusy] = useState(false);
  const [open, setOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const toggle = async () => {
    if (open) {
      setOpen(false);
      return;
    }
    setOpen(true);
    if (!card && !busy) {
      setBusy(true);
      setError(null);
      try {
        setCard(await cardExplain(term));
      } catch (e) {
        setError(String(e));
      } finally {
        setBusy(false);
      }
    }
  };

  return (
    <>
      <button
        type="button"
        onClick={() => void toggle()}
        className="ml-1 inline-flex items-center gap-0.5 align-baseline text-xs text-accent hover:underline"
      >
        <HelpCircle className="h-3 w-3" /> Why?
      </button>
      {open && (
        <div className="mt-1 rounded-md border border-border bg-surface-2 p-2 text-xs">
          {busy && (
            <span className="flex items-center gap-1 text-fg-subtle">
              <Loader2 className="h-3 w-3 animate-spin" /> Explaining…
            </span>
          )}
          {error && (
            <span className="text-danger" role="alert">
              {error}
            </span>
          )}
          {card && (
            <div className="space-y-1 text-fg-muted">
              {card.what_md && (
                <p>
                  <span className="font-medium text-fg">{card.term}:</span> {card.what_md}
                </p>
              )}
              {card.why_md && <p className="text-fg-subtle">{card.why_md}</p>}
            </div>
          )}
        </div>
      )}
    </>
  );
}
