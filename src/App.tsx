import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AppInfo {
  name: string;
  version: string;
}

/**
 * Phase 1 / Task 1 shell. Its only job is to prove the frontend -> Rust
 * round-trip: it calls the `app_info` command and renders the value. The
 * themed token system (Task 2), the SQLite layer (Task 3), and the hamburger
 * nav (Task 4) build on top of this shell.
 */
function App() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    invoke<AppInfo>("app_info")
      .then((value) => {
        if (!cancelled) setInfo(value);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-3 p-8 text-center">
      <h1 className="text-3xl font-semibold tracking-tight">Review Helper</h1>

      {info && (
        <p className="text-sm opacity-70">
          {info.name} v{info.version} — backend connected
        </p>
      )}
      {!info && !error && (
        <p className="text-sm opacity-50">Connecting to backend…</p>
      )}
      {error && (
        <p className="text-sm opacity-80" role="alert">
          Backend unavailable: {error}
        </p>
      )}
    </main>
  );
}

export default App;
