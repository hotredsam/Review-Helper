// Build the design gallery (every screen + an 8-theme switcher, sample data, no
// Tauri) into ONE self-contained HTML file and write it to the Desktop. Open it
// in any browser — no server needed. Run: `npm run export:design`.
import { build } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { viteSingleFile } from "vite-plugin-singlefile";
import { fileURLToPath } from "node:url";
import { dirname, resolve, join } from "node:path";
import { homedir } from "node:os";
import { mkdirSync, copyFileSync } from "node:fs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = resolve(root, "dist-gallery");

await build({
  root,
  configFile: false, // ignore the Tauri dev config; this is a standalone bundle
  plugins: [react(), tailwindcss(), viteSingleFile()],
  build: {
    outDir,
    emptyOutDir: true,
    rollupOptions: { input: resolve(root, "gallery.html") },
  },
  logLevel: "warn",
});

const dest = join(homedir(), "Desktop", "Review-Helper-Design");
mkdirSync(dest, { recursive: true });
const out = join(dest, "Review-Helper-Design.html");
copyFileSync(join(outDir, "gallery.html"), out);

console.log(`\n✓ Design snapshot written to: ${out}`);
console.log("  Open it in any browser — every screen, switchable across all 8 themes.");
