// @ts-nocheck
/** Design gallery: the full app shell rendered with mock data, plus a floating
 *  control bar to flip through every screen across all 8 themes. Built to a
 *  single self-contained HTML file (Phase H design export) you can open from the
 *  Desktop in any browser — no Tauri, no server. The control bar uses fixed
 *  inline styles (not theme tokens) so it stays legible over every theme. */
import { useEffect, useState } from "react";
import App from "../App";
import { THEMES } from "../theme/themes";
import { useThemeStore } from "../theme/themeStore";
import { useUiStore } from "../store/uiStore";
import { useLearningStore } from "../store/learningStore";

const SCREENS = [
  { key: "overview", label: "Overview", mode: "code", section: "overview" },
  { key: "understand", label: "Understand", mode: "code", section: "understand" },
  { key: "grill", label: "Grill", mode: "code", section: "grill" },
  { key: "chat", label: "Chat", mode: "code", section: "chat" },
  { key: "inbox", label: "Inbox", mode: "code", section: "inbox" },
  { key: "stack", label: "Stack", mode: "code", section: "stack" },
  { key: "decisions", label: "Decisions", mode: "code", section: "decisions" },
  { key: "plan", label: "Plan", mode: "code", section: "plan" },
  { key: "palette", label: "Palette", mode: "code", section: "palette" },
  { key: "settings", label: "Settings", mode: "code", section: "settings" },
  { key: "learn", label: "Learn", mode: "learning", subject: null },
  { key: "learn-study", label: "Learn · Study", mode: "learning", subject: 1 },
];

function applyScreen(s) {
  if (s.mode === "learning") {
    useLearningStore.getState().select(s.subject ?? null);
    useUiStore.setState({ appMode: "learning", sidebarCollapsed: false });
  } else {
    useUiStore.setState({ appMode: "code", activeSection: s.section, sidebarCollapsed: false });
  }
}

const BAR = {
  position: "fixed",
  left: "50%",
  bottom: 16,
  transform: "translateX(-50%)",
  zIndex: 9999,
  display: "flex",
  flexDirection: "column",
  gap: 8,
  maxWidth: "94vw",
  padding: "10px 12px",
  borderRadius: 14,
  background: "rgba(17,17,19,0.92)",
  color: "#fff",
  boxShadow: "0 10px 30px rgba(0,0,0,0.35)",
  backdropFilter: "blur(8px)",
  fontFamily: "system-ui, -apple-system, sans-serif",
  fontSize: 12,
};
const ROW = { display: "flex", gap: 6, flexWrap: "wrap", justifyContent: "center" };
const chip = (on) => ({
  padding: "4px 10px",
  borderRadius: 8,
  border: on ? "1px solid #fff" : "1px solid rgba(255,255,255,0.18)",
  background: on ? "#2563eb" : "transparent",
  color: "#fff",
  cursor: "pointer",
  whiteSpace: "nowrap",
});

export function Gallery() {
  const [screenKey, setScreenKey] = useState(SCREENS[0].key);
  const [theme, setTheme] = useState("light");

  useEffect(() => {
    applyScreen(SCREENS.find((s) => s.key === screenKey));
  }, [screenKey]);
  useEffect(() => {
    useThemeStore.getState().setTheme(theme);
  }, [theme]);

  return (
    <div style={{ position: "relative", height: "100vh", width: "100vw", overflow: "hidden" }}>
      <App />
      <div style={BAR} role="toolbar" aria-label="Design gallery controls">
        <div style={ROW}>
          {SCREENS.map((s) => (
            <button key={s.key} style={chip(s.key === screenKey)} onClick={() => setScreenKey(s.key)}>
              {s.label}
            </button>
          ))}
        </div>
        <div style={{ ...ROW, borderTop: "1px solid rgba(255,255,255,0.12)", paddingTop: 8 }}>
          <span style={{ alignSelf: "center", opacity: 0.6, marginRight: 4 }}>Theme</span>
          {THEMES.map((t) => (
            <button key={t.id} style={chip(t.id === theme)} onClick={() => setTheme(t.id)}>
              {t.label}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
