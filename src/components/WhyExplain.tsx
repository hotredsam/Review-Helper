import { useState } from "react";
import { Loader2, HelpCircle } from "lucide-react";
import { cardExplain, type Card } from "../api/cards";

/** Keep short choices verbatim; for a long composite choice (e.g. a full pipes
 *  stack string) explain just the leading technology so the term stays under the
 *  card length cap and "Why?" never errors. */
function conciseTerm(s: string): string {
  const t = s.trim();
  if (t.length <= 80) return t;
  const head = t.split(/[(/;,\n]| [-–—] /)[0].trim();
  return (head.length >= 3 ? head : t).slice(0, 120).trim();
}

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
        setCard(await cardExplain(conciseTerm(term)));
      } catch (e) {
        setError(String(e));
      } finally {
        setBusy(false);
      }
    }
  };

  const explainId = `explain-${term.toLowerCase().replace(/\s+/g, "-")}`;

  return (
    <>
      <button
        type="button"
        onClick={() => void toggle()}
        disabled={busy}
        aria-expanded={open}
        aria-controls={explainId}
        aria-label={`Why: ${term}`}
        className="ml-1 inline-flex items-center gap-0.5 align-baseline py-1 text-xs text-accent hover:underline disabled:opacity-60"
      >
        <HelpCircle className="h-3 w-3" /> Why?
      </button>
      {open && (
        <div id={explainId} className="mt-1 rounded-md border border-border bg-surface-2 p-2 text-xs">
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
