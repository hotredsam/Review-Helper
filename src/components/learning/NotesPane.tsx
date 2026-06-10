import { useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { learningNotes } from "../../api/learning";
import { MarkdownBlock } from "../MarkdownBlock";
import { modelStop } from "../../api/model";

/** A module's study notes (generated on first open, cached after). */
export function NotesPane({ moduleId }: { moduleId: number }) {
  const [md, setMd] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    setMd(null);
    setError(null);
    learningNotes(moduleId)
      .then((b) => live && setMd(b))
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
  if (md === null) {
    return (
      <p className="flex items-center gap-2 text-sm text-fg-subtle">
        <Loader2 className="h-4 w-4 animate-spin" /> Writing your notes…
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
  return (
    <div className="rounded-xl border border-border bg-surface p-5">
      <MarkdownBlock>{md}</MarkdownBlock>
    </div>
  );
}
