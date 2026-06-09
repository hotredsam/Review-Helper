// @ts-nocheck
/** Design-gallery entry. Installs the mock Tauri bridge BEFORE the app loads so
 *  every pane renders with sample data, then mounts the Gallery (all screens +
 *  an 8-theme switcher). Bundled to a single self-contained HTML by
 *  scripts/export-design.mjs. */
import { installMock } from "./mockInvoke";
installMock();
import "../index.css";
import { createRoot } from "react-dom/client";

localStorage.setItem("rh.tour.seen", "1"); // never show the first-run tour here

import("./Gallery").then(({ Gallery }) => {
  createRoot(document.getElementById("root")).render(<Gallery />);
});
