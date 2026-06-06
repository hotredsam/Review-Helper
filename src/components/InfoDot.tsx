import { useState } from "react";
import { HelpCircle } from "lucide-react";

/** A small "?" that explains a jargon term on click — wired inline so the UI
 *  never dead-ends on unfamiliar vocabulary. */
export function InfoDot({ term, explanation }: { term: string; explanation: string }) {
  const [open, setOpen] = useState(false);
  return (
    <span className="relative inline-block">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        aria-label={`What is ${term}?`}
        aria-expanded={open}
        className="ml-1 inline-flex align-middle text-fg-subtle hover:text-accent"
      >
        <HelpCircle className="h-3.5 w-3.5" />
      </button>
      {open && (
        <span
          role="tooltip"
          className="absolute left-0 top-6 z-20 block w-56 rounded-md border border-border bg-surface p-2 text-left text-xs font-normal normal-case tracking-normal text-fg-muted shadow-lg"
        >
          {explanation}
        </span>
      )}
    </span>
  );
}
