import { useEffect } from "react";
import { LogIn, LogOut, RefreshCw } from "lucide-react";
import { useGithubStore } from "../store/githubStore";

/** GitHub connection panel (Settings): connect via the gh CLI, show the login,
 *  list repositories, and sign out (which clears the keychain token). */
export function GithubConnect() {
  const {
    status,
    repos,
    connecting,
    loadingRepos,
    error,
    refreshStatus,
    connect,
    signOut,
    loadRepos,
  } = useGithubStore();

  useEffect(() => {
    void refreshStatus();
  }, [refreshStatus]);

  if (!status) {
    return <p className="text-sm text-fg-subtle">Checking GitHub…</p>;
  }

  if (!status.connected) {
    return (
      <div className="space-y-2">
        <button
          type="button"
          onClick={() => void connect()}
          disabled={connecting}
          className="flex items-center gap-2 rounded-lg bg-accent px-3 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
        >
          <LogIn className="h-4 w-4" />
          {connecting ? "Connecting…" : "Connect GitHub"}
        </button>
        <p className="text-xs text-fg-subtle">
          Uses your existing gh CLI sign-in. The token is stored in your macOS Keychain — never on disk.
        </p>
        {error && (
          <p className="text-sm text-danger" role="alert">
            {error}
          </p>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-2">
        <p className="text-sm text-fg">
          Connected as <span className="font-medium">{status.login ?? "GitHub"}</span>
        </p>
        <button
          type="button"
          onClick={() => void signOut()}
          className="flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1 text-xs text-fg-muted hover:bg-surface-2"
        >
          <LogOut className="h-3.5 w-3.5" /> Sign out
        </button>
      </div>

      <button
        type="button"
        onClick={() => void loadRepos()}
        disabled={loadingRepos}
        className="flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1 text-xs text-fg-muted hover:bg-surface-2 disabled:opacity-60"
      >
        <RefreshCw className={"h-3.5 w-3.5 " + (loadingRepos ? "animate-spin" : "")} />
        {repos.length ? `${repos.length} repositories` : "Load repositories"}
      </button>

      {repos.length > 0 && (
        <ul className="max-h-40 overflow-auto rounded-lg border border-border bg-surface text-sm">
          {repos.slice(0, 50).map((r) => (
            <li
              key={r.full_name}
              className="flex items-center justify-between gap-2 border-b border-border px-3 py-1.5 last:border-0"
            >
              <span className="truncate text-fg">{r.full_name}</span>
              {r.private && (
                <span className="shrink-0 rounded bg-surface-2 px-1.5 py-0.5 text-xs text-fg-subtle">
                  private
                </span>
              )}
            </li>
          ))}
        </ul>
      )}

      {error && (
        <p className="text-sm text-danger" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
