// @ts-nocheck
/** QA preview entry — installs the mock Tauri bridge BEFORE the app loads, then
 *  mounts the chosen pane/theme from URL params (?pane=overview&theme=dark). */
import { installMock } from "./mockInvoke";
installMock();
import "../index.css";

const params = new URLSearchParams(location.search);
const theme = params.get("theme") || "light";

// Defer the app import until after the mock is installed; the theme store sets
// data-theme itself, so pass it through rather than setting the attribute here.
import("./mount").then(({ mount }) => mount(params.get("pane") || "overview", theme));
