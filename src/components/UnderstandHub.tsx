import { useEffect, useState, type FormEvent } from "react";
import { Search, Loader2, BookOpen } from "lucide-react";
import { cardsList, cardExplain, type Card } from "../api/cards";

const DOMAIN_ORDER = [
  "architecture",
  "frontend",
  "backend",
  "pipes",
  "deployment",
  "business",
  "design",
  "ux",
  "other",
];
const DOMAIN_LABEL: Record<string, string> = {
  architecture: "Architecture",
  frontend: "Frontend",
  backend: "Backend",
  pipes: "Pipes",
  deployment: "Deployment",
  business: "Business",
  design: "Design",
  ux: "UX",
  other: "Other",
};

/**
 * The Understand hub: browse the learning cards by domain, and "explain
 * anything" — type a term and the model generates + caches a card. Never
 * dead-ends: an unknown term always offers generation.
 */
export function UnderstandHub() {
  const [cards, setCards] = useState<Card[]>([]);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<Card | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void cardsList()
      .then(setCards)
      .catch((e) => setError(String(e)));
  }, []);

  const explain = async (term: string) => {
    const t = term.trim();
    if (!t || busy) return;
    setBusy(true);
    setError(null);
    try {
      const card = await cardExplain(t);
      setSelected(card);
      setCards(await cardsList());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const openCard = async (card: Card) => {
    if (card.what_md) {
      setSelected(card);
    } else {
      await explain(card.term); // detected stub → generate on demand
    }
  };

  const grouped = DOMAIN_ORDER.map((d) => ({
    domain: d,
    cards: cards.filter((c) => (c.domain ?? "other") === d),
  })).filter((g) => g.cards.length > 0);

  return (
    <div className="mx-auto max-w-4xl space-y-6 p-8">
      <form
        onSubmit={(e: FormEvent) => {
          e.preventDefault();
          void explain(query);
          setQuery("");
        }}
        className="flex gap-2"
      >
        <div className="relative flex-1">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-fg-subtle" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            maxLength={200}
            placeholder="Explain anything — a term, concept, or technology…"
            className="w-full rounded-lg border border-border bg-surface py-2 pl-9 pr-3 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none focus:ring-2 focus:ring-ring/40"
          />
        </div>
        <button
          type="submit"
          disabled={busy || !query.trim()}
          className="flex items-center gap-1.5 rounded-lg bg-accent px-3 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
        >
          {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <BookOpen className="h-4 w-4" />}
          Explain
        </button>
      </form>

      {error && (
        <p className="text-sm text-danger" role="alert">
          {error}
        </p>
      )}

      <div aria-live="polite" aria-atomic="true">
        {selected && <CardDetail card={selected} />}
      </div>

      <div className="space-y-4">
        {grouped.map((g) => (
          <section key={g.domain}>
            <h2 className="mb-2 text-xs font-semibold uppercase tracking-wide text-fg-subtle">
              {DOMAIN_LABEL[g.domain] ?? g.domain}
            </h2>
            <div className="flex flex-wrap gap-2">
              {g.cards.map((c) => (
                <button
                  key={c.id}
                  type="button"
                  onClick={() => void openCard(c)}
                  aria-label={c.what_md ? c.term : `${c.term} — generate explanation`}
                  className={
                    "rounded-full border px-3 py-1 text-sm transition-colors " +
                    (selected?.id === c.id
                      ? "border-accent bg-accent/10 text-fg"
                      : "border-border text-fg-muted hover:bg-surface-2 hover:text-fg")
                  }
                >
                  {c.term}
                  {!c.what_md && (
                    <span aria-hidden="true" className="ml-1 text-xs text-fg-subtle">
                      ·
                    </span>
                  )}
                </button>
              ))}
            </div>
          </section>
        ))}
        {cards.length === 0 && !error && <p className="text-sm text-fg-subtle">Loading cards…</p>}
      </div>
    </div>
  );
}

function CardDetail({ card }: { card: Card }) {
  return (
    <div className="rounded-xl border border-border bg-surface p-5">
      <div className="mb-3 flex items-center gap-2">
        <h2 className="text-lg font-semibold text-fg">{card.term}</h2>
        {card.domain && (
          <span className="rounded-full bg-surface-2 px-2 py-0.5 text-xs text-fg-muted">
            {card.domain}
          </span>
        )}
      </div>
      <CardSection label="What it is" body={card.what_md} />
      <CardSection label="When to use it" body={card.when_md} />
      <CardSection label="Why it matters" body={card.why_md} />
    </div>
  );
}

function CardSection({ label, body }: { label: string; body: string | null }) {
  if (!body) return null;
  return (
    <div className="mb-3 last:mb-0">
      <p className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">{label}</p>
      <p className="mt-0.5 whitespace-pre-wrap text-sm text-fg-muted">{body}</p>
    </div>
  );
}
