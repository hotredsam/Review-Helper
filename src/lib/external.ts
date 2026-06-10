import { openUrl } from "@tauri-apps/plugin-opener";

/** Open a link in the system browser — never let an in-app anchor replace the
 *  whole webview (audit A33: LLM-generated markdown links navigated the app). */
export function openExternal(url: string | undefined) {
  if (url && /^https?:\/\//i.test(url)) void openUrl(url).catch(() => {});
}
