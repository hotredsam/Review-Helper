import { useEffect, useMemo, useState, type FormEvent } from "react";
import { Search, Loader2, BookOpen, FolderGit2 } from "lucide-react";
import {
  cardsList,
  cardExplain,
  cardCleanTerm,
  cardProjectTerms,
  type Card,
} from "../api/cards";
import { CardChat } from "./CardChat";
import type { Project } from "../api/projects";

const DOMAIN_ORDER = ["architecture", "frontend", "backend", "pipes", "deployment", "business", "design", "ux", "other"];
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
 * The Understand hub: a compact, filterable card library. Type to filter live;
 * "Explain" generates a card for a term (spelling/grammar cleaned first). Filter
 * to a domain or to just this project's cards. Open a card to read it and chat
 * about the concept inline. Never dead-ends.
 */
export function UnderstandHub({ project }: { project: Project }) {
  const [cards, setCards] = useState<Card[]>([]);
  const [projectTerms, setProjectTerms] = useState<Set<string>>(new Set());
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<"all" | "project" | string>("all");
  const [selected, setSelected] = useState<Card | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = async () => {
    const forProject = project.id;
    const [cs, terms] = await Promise.all([cardsList(), cardProjectTerms(forProject)]);
    // A slow response from a previous project must not overwrite this one.
    if (forProject !== project.id) return;
    setCards(cs);
    setProjectTerms(new Set(terms.map((t) => t.toLowerCase())));
  };
  useEffect(() => {
    void refresh().catch((e) => setError(String(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [project.id]);

  // Generate a card for a typed term (spelling/grammar cleaned first).
  const explain = async (raw: string, clean: boolean) => {
    const t = raw.trim();
    if (!t || busy) return;
    setBusy(true);
    setError(null);
    try {
      const term = clean ? await cardCleanTerm(t).catch(() => t) : t;
      const card = await cardExplain(term, project.id);
      setSelected(card);
      setQuery("");
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const openCard = async (card: Card) => {
    if (card.what_md) setSelected(card);
    else await explain(card.term, false); // detected stub → generate on demand
  };

  const filtered = useMemo(() => {
    let cs = cards;
    if (filter === "project") cs = cs.filter((c) => projectTerms.has(c.term.toLowerCase()));
    else if (filter !== "all") cs = cs.filter((c) => (c.domain ?? "other") === filter);
    const q = query.trim().toLowerCase();
    if (q) cs = cs.filter((c) => c.term.toLowerCase().includes(q));
    return cs;
  }, [cards, filter, projectTerms, query]);

  const grouped = DOMAIN_ORDER.map((d) => ({ domain: d, cards: filtered.filter((c) => (c.domain ?? "other") === d) })).filter(
    (g) => g.cards.length > 0,
  );

  const FILTERS: { id: "all" | "project" | string; label: string; icon?: boolean }[] = [
    { id: "all", label: "All" },
    { id: "project", label: "This project", icon: true },
    ...DOMAIN_ORDER.filter((d) => cards.some((c) => (c.domain ?? "other") === d)).map((d) => ({ id: d, label: DOMAIN_LABEL[d] })),
  ];

  return (
    <div className="mx-auto max-w-4xl space-y-4 p-8">
      <form
        onSubmit={(e: FormEvent) => {
          e.preventDefault();
          void explain(query, true);
        }}
        className="flex gap-2"
      >
        <div className="relative flex-1">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-fg-subtle" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            maxLength={200}
            placeholder="Filter cards, or explain anything — a term, concept, or technology…"
            aria-label="Filter or explain a term"
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

      {/* Filters */}
      <div className="flex flex-wrap gap-1.5">
        {FILTERS.map((f) => {
          const active = filter === f.id;
          return (
            <button
              key={f.id}
              type="button"
              onClick={() => setFilter(f.id)}
              className={
                "flex items-center gap-1 rounded-full px-2.5 py-1 text-xs font-medium transition-colors " +
                (active ? "bg-accent text-accent-fg" : "border border-border text-fg-muted hover:bg-surface-2 hover:text-fg")
              }
            >
              {f.icon && <FolderGit2 className="h-3 w-3" />}
              {f.label}
            </button>
          );
        })}
      </div>

      {error && (
        <p className="text-sm text-danger" role="alert">
          {error}
        </p>
      )}

      <div aria-live="polite" aria-atomic="true">
        {selected && <CardDetail card={selected} project={project.id} />}
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
                </button>
              ))}
            </div>
          </section>
        ))}
        {cards.length === 0 && !error && <p className="text-sm text-fg-subtle">Loading cards…</p>}
        {cards.length > 0 && filtered.length === 0 && (
          <p className="text-sm text-fg-subtle">
            No cards match. {filter === "project" ? "Explain a term to add it to this project." : "Try a different filter."}
          </p>
        )}
      </div>
    </div>
  );
}

function CardDetail({ card, project }: { card: Card; project: number }) {
  return (
    <div className="rounded-xl border border-border bg-surface p-5">
      <div className="mb-3 flex items-center gap-2">
        <h2 className="text-lg font-semibold text-fg">{card.term}</h2>
        {card.domain && (
          <span className="rounded-full bg-surface-2 px-2 py-0.5 text-xs text-fg-muted">
            {DOMAIN_LABEL[card.domain] ?? card.domain}
          </span>
        )}
      </div>
      <CardSection label="What it is" body={card.what_md} />
      <CardSection label="When to use it" body={card.when_md} />
      <CardSection label="Why it matters" body={card.why_md} />
      <CardChat project={project} term={card.term} />
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
