import { useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { learningNotes } from "../../api/learning";
import { MarkdownBlock } from "../MarkdownBlock";

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
      </p>
    );
  }
  return (
    <div className="rounded-xl border border-border bg-surface p-5">
      <MarkdownBlock>{md}</MarkdownBlock>
    </div>
  );
}
