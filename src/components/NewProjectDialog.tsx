import { useState, type FormEvent } from "react";
import { Modal } from "./Modal";
import { useProjectStore } from "../store/projectStore";
import type { Project } from "../api/projects";

interface Props {
  open: boolean;
  onClose: () => void;
}

/** Create a project. Delegates persistence to the store -> Rust command, and
 *  surfaces validation errors (empty name, etc.) returned from the backend. */
export function NewProjectDialog({ open, onClose }: Props) {
  const create = useProjectStore((s) => s.create);
  const [name, setName] = useState("");
  const [kind, setKind] = useState<Project["kind"]>("new");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const close = () => {
    setName("");
    setKind("new");
    setError(null);
    setBusy(false);
    onClose();
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await create(name, kind);
      close();
    } catch (err) {
      setError(String(err));
      setBusy(false);
    }
  };

  return (
    <Modal open={open} onClose={close} title="New project">
      <form onSubmit={submit} className="space-y-4">
        <label className="block space-y-1.5">
          <span className="text-sm font-medium text-fg">Name</span>
          <input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="My app"
            className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none focus:ring-2 focus:ring-ring/40"
          />
        </label>

        <fieldset className="space-y-1.5">
          <span className="text-sm font-medium text-fg">Type</span>
          <div className="flex gap-2">
            {(["new", "imported"] as const).map((k) => (
              <button
                key={k}
                type="button"
                onClick={() => setKind(k)}
                className={
                  "flex-1 rounded-lg border px-3 py-2 text-sm transition-colors " +
                  (kind === k
                    ? "border-accent bg-accent/10 text-fg"
                    : "border-border text-fg-muted hover:bg-surface-2")
                }
              >
                {k === "new" ? "New build" : "Imported"}
              </button>
            ))}
          </div>
        </fieldset>

        {error && (
          <p className="text-sm text-danger" role="alert">
            {error}
          </p>
        )}

        <div className="flex justify-end gap-2 pt-1">
          <button
            type="button"
            onClick={close}
            className="rounded-lg px-3 py-2 text-sm font-medium text-fg-muted hover:bg-surface-2"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={busy}
            className="rounded-lg bg-accent px-4 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
          >
            {busy ? "Creating…" : "Create"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
