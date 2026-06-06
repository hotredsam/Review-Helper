import { ThemeSwitcher } from "./ThemeSwitcher";

/** Minimal Settings pane. Theme is functional now; the model provider and other
 *  options arrive in later phases. */
export function SettingsView() {
  return (
    <div className="mx-auto max-w-xl space-y-8 p-8">
      <section className="space-y-3">
        <div>
          <h2 className="text-sm font-semibold text-fg">Theme</h2>
          <p className="text-sm text-fg-muted">
            Choose how Review Helper looks. Your choice is saved across restarts.
          </p>
        </div>
        <ThemeSwitcher />
      </section>

      <section className="space-y-2">
        <h2 className="text-sm font-semibold text-fg">Model provider</h2>
        <p className="text-sm text-fg-muted">
          Connecting Claude and configuring local / credit options lands in the next phase.
        </p>
      </section>
    </div>
  );
}
