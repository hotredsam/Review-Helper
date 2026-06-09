// @ts-nocheck
import { createRoot } from "react-dom/client";
import App from "../App";
import { useUiStore } from "../store/uiStore";
import { useThemeStore } from "../theme/themeStore";

/** Mount the full app shell with the requested section + theme active (mock data
 *  fills the stores via the installed Tauri mock). */
export function mount(pane: string, theme: string) {
  localStorage.setItem("rh.tour.seen", "1"); // skip the first-run tour
  useThemeStore.getState().setTheme(theme); // applies data-theme via the store subscription
  const text = new URLSearchParams(location.search).get("text");
  useUiStore.setState({
    activeSection: pane,
    sidebarCollapsed: false,
    textMode: text === "technical" ? "technical" : "easy",
  });
  createRoot(document.getElementById("root")!).render(<App />);
}
