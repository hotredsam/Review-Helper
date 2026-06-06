import { useEffect, useId, useRef, useState } from "react";
import { HelpCircle } from "lucide-react";

/** A small "?" that explains a jargon term on click — wired inline so the UI
 *  never dead-ends on unfamiliar vocabulary. Dismisses on Escape or click-out. */
export function InfoDot({ term, explanation }: { term: string; explanation: string }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLSpanElement>(null);
  const id = useId();

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <span ref={ref} className="relative inline-block">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        aria-label={`What is ${term}?`}
        aria-expanded={open}
        aria-controls={id}
        className="ml-1 inline-flex align-middle text-fg-subtle hover:text-accent"
      >
        <HelpCircle className="h-3.5 w-3.5" />
      </button>
      {open && (
        <span
          id={id}
          role="tooltip"
          className="absolute left-0 top-6 z-20 block w-56 rounded-md border border-border bg-surface p-2 text-left text-xs font-normal normal-case tracking-normal text-fg-muted shadow-lg"
        >
          {explanation}
        </span>
      )}
    </span>
  );
}
