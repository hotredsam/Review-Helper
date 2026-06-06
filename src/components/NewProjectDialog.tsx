import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import { RefreshCw } from "lucide-react";
import { Modal } from "./Modal";
import { useProjectStore } from "../store/projectStore";
import { useGithubStore } from "../store/githubStore";

type Mode = "blank" | "import" | "link" | "github";

const TABS: { id: Mode; label: string }[] = [
  { id: "blank", label: "Blank" },
  { id: "import", label: "Import" },
  { id: "link", label: "Link URL" },
  { id: "github", label: "GitHub" },
];

const inputCls =
  "w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none focus:ring-2 focus:ring-ring/40";

interface Props {
  open: boolean;
  onClose: () => void;
}

/**
 * Four ways to add a project: a blank project, import a repo from your GitHub,
 * link a repo by URL, or create a brand-new repo on GitHub. The GitHub paths go
 * through the projectStore, which calls the Rust commands.
 */
export function NewProjectDialog({ open, onClose }: Props) {
  const create = useProjectStore((s) => s.create);
  const importRepo = useProjectStore((s) => s.importRepo);
  const linkUrl = useProjectStore((s) => s.linkUrl);
  const createRepo = useProjectStore((s) => s.createRepo);

  const ghStatus = useGithubStore((s) => s.status);
  const repos = useGithubStore((s) => s.repos);
  const loadRepos = useGithubStore((s) => s.loadRepos);
  const loadingRepos = useGithubStore((s) => s.loadingRepos);
  const connected = ghStatus?.connected ?? false;

  const [mode, setMode] = useState<Mode>("blank");
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const [isPrivate, setIsPrivate] = useState(true);
  const [search, setSearch] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const close = () => {
    setMode("blank");
    setName("");
    setUrl("");
    setSearch("");
    setIsPrivate(true);
    setBusy(false);
    setError(null);
    onClose();
  };

  const run = async (fn: () => Promise<unknown>) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await fn();
      close();
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  // Auto-load repos when the Import tab is active and connected.
  useEffect(() => {
    if (open && mode === "import" && connected && repos.length === 0 && !loadingRepos) {
      void loadRepos();
    }
  }, [open, mode, connected, repos.length, loadingRepos, loadRepos]);

  const filtered = repos.filter((r) =>
    r.full_name.toLowerCase().includes(search.toLowerCase()),
  );

  return (
    <Modal open={open} onClose={close} title="New project">
      <div className="space-y-4">
        <div className="grid grid-cols-4 gap-1 rounded-lg border border-border bg-surface p-1">
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              onClick={() => {
                setMode(t.id);
                setError(null);
              }}
              className={
                "rounded-md px-2 py-1.5 text-xs font-medium transition-colors " +
                (mode === t.id
                  ? "bg-accent text-accent-fg"
                  : "text-fg-muted hover:bg-surface-2 hover:text-fg")
              }
            >
              {t.label}
            </button>
          ))}
        </div>

        {mode === "blank" && (
          <form
            onSubmit={(e: FormEvent) => {
              e.preventDefault();
              void run(() => create(name, "new"));
            }}
            className="space-y-3"
          >
            <Field label="Name">
              <input
                autoFocus
                className={inputCls}
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="My app"
              />
            </Field>
            <p className="text-xs text-fg-subtle">A blank project, not linked to GitHub.</p>
            <Actions busy={busy} onCancel={close} label="Create" />
          </form>
        )}

        {mode === "link" && (
          <form
            onSubmit={(e: FormEvent) => {
              e.preventDefault();
              void run(() => linkUrl(url));
            }}
            className="space-y-3"
          >
            <Field label="Repository URL">
              <input
                autoFocus
                className={inputCls}
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder="https://github.com/owner/repo"
              />
            </Field>
            {!connected && <ConnectHint />}
            <Actions busy={busy} onCancel={close} label="Link repo" disabled={!connected} />
          </form>
        )}

        {mode === "github" && (
          <form
            onSubmit={(e: FormEvent) => {
              e.preventDefault();
              void run(() => createRepo(name, isPrivate));
            }}
            className="space-y-3"
          >
            <Field label="Repository name">
              <input
                autoFocus
                className={inputCls}
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="my-new-repo"
              />
            </Field>
            <label className="flex items-center gap-2 text-sm text-fg">
              <input
                type="checkbox"
                checked={isPrivate}
                onChange={(e) => setIsPrivate(e.target.checked)}
                className="h-4 w-4 accent-accent"
              />
              Private repository
            </label>
            <p className="text-xs text-fg-subtle">
              Creates a real, empty repository on your GitHub account.
            </p>
            {!connected && <ConnectHint />}
            <Actions busy={busy} onCancel={close} label="Create on GitHub" disabled={!connected} />
          </form>
        )}

        {mode === "import" &&
          (connected ? (
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <input
                  className={inputCls}
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  placeholder="Search your repositories…"
                />
                <button
                  type="button"
                  onClick={() => void loadRepos()}
                  disabled={loadingRepos}
                  title="Reload"
                  aria-label="Reload repositories"
                  className="shrink-0 rounded-lg border border-border px-2.5 py-2 text-fg-muted hover:bg-surface-2 disabled:opacity-60"
                >
                  <RefreshCw className={"h-4 w-4 " + (loadingRepos ? "animate-spin" : "")} />
                </button>
              </div>
              <ul className="max-h-56 overflow-auto rounded-lg border border-border bg-surface">
                {filtered.length === 0 && (
                  <li className="px-3 py-3 text-sm text-fg-subtle">
                    {loadingRepos
                      ? "Loading…"
                      : repos.length === 0
                        ? "No repositories loaded."
                        : "No matches."}
                  </li>
                )}
                {filtered.slice(0, 100).map((r) => (
                  <li key={r.full_name}>
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => void run(() => importRepo(r))}
                      className="flex w-full items-center justify-between gap-2 border-b border-border px-3 py-2 text-left text-sm last:border-0 hover:bg-surface-2 disabled:opacity-60"
                    >
                      <span className="truncate text-fg">{r.full_name}</span>
                      {r.private && (
                        <span className="shrink-0 rounded bg-surface-2 px-1.5 py-0.5 text-xs text-fg-subtle">
                          private
                        </span>
                      )}
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          ) : (
            <div className="space-y-3">
              <ConnectHint />
              <div className="flex justify-end">
                <CancelButton onCancel={close} />
              </div>
            </div>
          ))}

        {error && (
          <p className="text-sm text-danger" role="alert">
            {error}
          </p>
        )}
      </div>
    </Modal>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="block space-y-1.5">
      <span className="text-sm font-medium text-fg">{label}</span>
      {children}
    </label>
  );
}

function CancelButton({ onCancel }: { onCancel: () => void }) {
  return (
    <button
      type="button"
      onClick={onCancel}
      className="rounded-lg px-3 py-2 text-sm font-medium text-fg-muted hover:bg-surface-2"
    >
      Cancel
    </button>
  );
}

function Actions({
  busy,
  onCancel,
  label,
  disabled,
}: {
  busy: boolean;
  onCancel: () => void;
  label: string;
  disabled?: boolean;
}) {
  return (
    <div className="flex justify-end gap-2 pt-1">
      <CancelButton onCancel={onCancel} />
      <button
        type="submit"
        disabled={busy || disabled}
        className="rounded-lg bg-accent px-4 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
      >
        {busy ? "Working…" : label}
      </button>
    </div>
  );
}

function ConnectHint() {
  const connect = useGithubStore((s) => s.connect);
  const connecting = useGithubStore((s) => s.connecting);
  return (
    <div className="rounded-lg border border-border bg-surface-2 p-3 text-xs text-fg-muted">
      <p className="mb-2">Connect GitHub to use this option.</p>
      <button
        type="button"
        onClick={() => void connect()}
        disabled={connecting}
        className="rounded-md bg-accent px-2.5 py-1 text-xs font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
      >
        {connecting ? "Connecting…" : "Connect GitHub"}
      </button>
    </div>
  );
}
