import { ThemeSwitcher } from "./ThemeSwitcher";
import { ModelConsole } from "./ModelConsole";
import { ProviderSettings } from "./ProviderSettings";
import { ModelDebug } from "./ModelDebug";
import { GithubConnect } from "./GithubConnect";

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

      <section className="space-y-3">
        <div>
          <h2 className="text-sm font-semibold text-fg">GitHub</h2>
          <p className="text-sm text-fg-muted">
            Connect GitHub to import, link, and create repositories for your projects.
          </p>
        </div>
        <GithubConnect />
      </section>

      <section className="space-y-4">
        <div>
          <h2 className="text-sm font-semibold text-fg">Model</h2>
          <p className="text-sm text-fg-muted">
            Review Helper drives the Claude Code CLI for all planning. Test the connection below.
          </p>
        </div>
        <ProviderSettings />
        <ModelConsole />
        <ModelDebug />
      </section>
    </div>
  );
}
