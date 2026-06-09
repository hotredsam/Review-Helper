import { useState, type ChangeEvent } from "react";
import { Modal } from "../Modal";
import { useLearningStore } from "../../store/learningStore";

type Kind = "describe" | "upload";

/** Create a study subject by describing a goal or uploading material. Upload
 *  reads plain-text / markdown files in the browser; richer formats (PDF) are
 *  ingested server-side later. The model later grills the user on scope, so a
 *  short description is enough to start. */
export function NewSubjectDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const create = useLearningStore((s) => s.create);
  const [kind, setKind] = useState<Kind>("describe");
  const [title, setTitle] = useState("");
  const [goal, setGoal] = useState("");
  const [uploadText, setUploadText] = useState("");
  const [fileName, setFileName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reset = () => {
    setKind("describe");
    setTitle("");
    setGoal("");
    setUploadText("");
    setFileName("");
    setError(null);
  };
  const close = () => {
    reset();
    onClose();
  };

  const onFile = async (e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setError(null);
    if (file.size > 5_000_000) {
      setError("That file is large (over 5 MB). Upload a smaller text/markdown file, or describe the subject instead.");
      return;
    }
    try {
      const text = await file.text();
      setUploadText(text);
      setFileName(file.name);
      if (!title.trim()) setTitle(file.name.replace(/\.[^.]+$/, ""));
    } catch {
      setError("Couldn't read that file. Try a plain-text or markdown file, or describe the subject instead.");
    }
  };

  const submit = async () => {
    const t = title.trim();
    if (!t) {
      setError("Give the subject a name.");
      return;
    }
    const sourceText = kind === "describe" ? goal.trim() : uploadText.trim();
    if (kind === "describe" && !sourceText) {
      setError("Describe what you want to learn.");
      return;
    }
    if (kind === "upload" && !sourceText) {
      setError("Pick a text or markdown file to learn from.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await create(t, kind, sourceText);
      close();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const TABS: { id: Kind; label: string }[] = [
    { id: "describe", label: "Describe" },
    { id: "upload", label: "Upload" },
  ];

  return (
    <Modal open={open} onClose={close} title="New subject">
      <div className="space-y-3">
        <input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          maxLength={200}
          placeholder="Subject name (e.g. Spanish A1, Linear algebra)"
          aria-label="Subject name"
          className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none focus:ring-2 focus:ring-ring/40"
        />

        <div role="radiogroup" aria-label="Source" className="inline-flex rounded-lg border border-border bg-surface p-0.5 text-xs">
          {TABS.map((tab) => {
            const active = tab.id === kind;
            return (
              <button
                key={tab.id}
                type="button"
                role="radio"
                aria-checked={active}
                onClick={() => setKind(tab.id)}
                className={
                  "rounded-md px-3 py-1 font-medium transition-colors " +
                  (active ? "bg-accent text-accent-fg" : "text-fg-muted hover:bg-surface-2 hover:text-fg")
                }
              >
                {tab.label}
              </button>
            );
          })}
        </div>

        {kind === "describe" ? (
          <textarea
            value={goal}
            onChange={(e) => setGoal(e.target.value)}
            maxLength={40_000}
            rows={4}
            placeholder="What do you want to learn, and why? (It'll grill you on the specifics next.)"
            aria-label="What you want to learn"
            className="w-full resize-y rounded-lg border border-border bg-surface px-3 py-2 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none focus:ring-2 focus:ring-ring/40"
          />
        ) : (
          <div className="space-y-2">
            <input
              type="file"
              accept=".txt,.md,.markdown,text/plain,text/markdown"
              onChange={onFile}
              aria-label="Upload material"
              className="block w-full text-sm text-fg-muted file:mr-3 file:rounded-md file:border file:border-border file:bg-surface file:px-3 file:py-1.5 file:text-sm file:text-fg hover:file:bg-surface-2"
            />
            {fileName && (
              <p className="text-xs text-fg-subtle">
                {fileName} — {uploadText.length.toLocaleString()} characters loaded.
              </p>
            )}
            <p className="text-xs text-fg-subtle">Text or markdown for now. PDF support is coming.</p>
          </div>
        )}

        {error && (
          <p className="text-sm text-danger" role="alert">
            {error}
          </p>
        )}

        <div className="flex justify-end gap-2 pt-1">
          <button onClick={close} className="rounded-lg border border-border px-3 py-1.5 text-sm text-fg-muted hover:bg-surface-2">
            Cancel
          </button>
          <button
            onClick={() => void submit()}
            disabled={busy}
            className="rounded-lg bg-accent px-3 py-1.5 text-sm font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
          >
            {busy ? "Creating…" : "Create"}
          </button>
        </div>
      </div>
    </Modal>
  );
}
