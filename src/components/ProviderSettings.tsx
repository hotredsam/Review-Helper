import { useEffect, useState } from "react";
import {
  getModelConfig,
  setModelConfig,
  type ModelConfig,
  type ProviderKind,
} from "../api/settings";
import { useUiStore } from "../store/uiStore";

/**
 * Model-provider configuration. Claude is the default and routes real calls;
 * Local is an off-by-default stub; the API-credit toggle is reserved for future
 * routing. Changes persist to the SQLite settings table via the backend.
 */
export function ProviderSettings() {
  const [config, setConfig] = useState<ModelConfig | null>(null);
  const [error, setError] = useState<string | null>(null); // load failure only
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    getModelConfig()
      .then(setConfig)
      .catch((e) => setError(String(e)));
  }, []);

  const update = (patch: Partial<ModelConfig>) => {
    if (!config) return;
    const next = { ...config, ...patch };
    setConfig(next);
    setSaveError(null);
    setModelConfig(next)
      .then(() => useUiStore.getState().setNotice("Model settings saved."))
      .catch((e) => {
        // The save failed: say so accurately, keep the panel usable, and
        // re-sync from the backend so the screen matches what's persisted.
        setSaveError(String(e));
        getModelConfig().then(setConfig).catch(() => {});
      });
  };

  if (error) {
    return (
      <p className="text-sm text-danger" role="alert">
        Couldn't load model settings: {error}
      </p>
    );
  }
  if (!config) {
    return <p className="text-sm text-fg-subtle">Loading…</p>;
  }

  return (
    <div className="space-y-4">
      {saveError && (
        <p className="text-sm text-danger" role="alert">
          Couldn't save model settings: {saveError}
        </p>
      )}
      {config.provider === "local" && (
        <p className="rounded-md border border-border bg-surface-2 px-3 py-2 text-xs text-fg-muted" role="note">
          The local provider is a stub: chat and every generator (plan, grill, learning, cards, assess)
          will show a "configure me" notice instead of calling a model. Switch to Claude for real output —
          nothing spends Claude credits while Local is selected.
        </p>
      )}
      <div
        role="radiogroup"
        aria-label="Provider"
        className="inline-flex gap-1 rounded-lg border border-border bg-surface p-1"
      >
        {(["claude", "local"] as ProviderKind[]).map((p) => {
          const active = config.provider === p;
          return (
            <button
              key={p}
              type="button"
              role="radio"
              aria-checked={active}
              onClick={() => update({ provider: p })}
              className={
                "rounded-md px-3 py-1.5 text-sm font-medium transition-colors " +
                (active
                  ? "bg-accent text-accent-fg"
                  : "text-fg-muted hover:bg-surface-2 hover:text-fg")
              }
            >
              {p === "claude" ? "Claude Code" : "Local (stub)"}
            </button>
          );
        })}
      </div>

      {config.provider === "local" && (
        <div className="space-y-1.5">
          <label className="block text-sm font-medium text-fg" htmlFor="local-endpoint">
            Local endpoint
          </label>
          <input
            id="local-endpoint"
            value={config.local_endpoint ?? ""}
            onChange={(e) => update({ local_endpoint: e.target.value || null })}
            placeholder="http://localhost:11434/v1"
            className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none focus:ring-2 focus:ring-ring/40"
          />
          <p className="text-xs text-fg-subtle">
            Stub in v1 — selecting Local returns a notice instead of running calls.
          </p>
        </div>
      )}

      <label className="flex items-center justify-between gap-3">
        <span className="text-sm text-fg">
          API-credit overflow
          <span className="block text-xs text-fg-subtle">
            Off by default. Reserved for future routing — no effect in v1.
          </span>
        </span>
        <input
          type="checkbox"
          checked={config.api_credit_overflow}
          onChange={(e) => update({ api_credit_overflow: e.target.checked })}
          className="h-4 w-4 accent-accent"
        />
      </label>
    </div>
  );
}
