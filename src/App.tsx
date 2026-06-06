import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ThemeSwitcher } from "./components/ThemeSwitcher";

interface AppInfo {
  name: string;
  version: string;
}

/**
 * Phase 1 shell. Proves the frontend -> Rust round-trip (the `app_info`
 * command) and hosts the theme switcher. The SQLite layer (Task 3) and the
 * hamburger nav (Task 4) build on top of this shell. Styled entirely with
 * theme tokens — no hardcoded colors.
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
    <main className="flex min-h-screen flex-col items-center justify-center gap-6 bg-bg p-8 text-center text-fg">
      <div className="flex flex-col items-center gap-2">
        <h1 className="text-3xl font-semibold tracking-tight">Review Helper</h1>
        {info && (
          <p className="text-sm text-fg-muted">
            {info.name} v{info.version} — backend connected
          </p>
        )}
        {!info && !error && (
          <p className="text-sm text-fg-subtle">Connecting to backend…</p>
        )}
        {error && (
          <p className="text-sm text-danger" role="alert">
            Backend unavailable: {error}
          </p>
        )}
      </div>

      <ThemeSwitcher />
    </main>
  );
}

export default App;
