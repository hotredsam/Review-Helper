// QA-only: capture pane screenshots from the Vite-served preview page using a
// headless Chromium (Playwright). Requires `npm run dev` running on :1420.
//   node scripts/shoot.mjs <outDir> <panes csv> <themes csv>
import { chromium } from "playwright";
import { mkdirSync } from "node:fs";

const OUT = process.argv[2] || "docs/ui-shots";
const PANES = (process.argv[3] || "overview,understand,grill,chat,stack,decisions,inbox,plan,palette,learn").split(",");
const THEMES = (process.argv[4] || "light,dark").split(",");
const BASE = "http://localhost:1420/preview.html";

mkdirSync(OUT, { recursive: true });
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1200, height: 800 }, deviceScaleFactor: 2 });

for (const pane of PANES) {
  for (const theme of THEMES) {
    const url = `${BASE}?pane=${pane}&theme=${theme}`;
    try {
      await page.goto(url, { waitUntil: "domcontentloaded", timeout: 30000 });
      await page.waitForTimeout(1200); // let async mock store-loads settle
      await page.screenshot({ path: `${OUT}/${pane}-${theme}.png` });
      console.log("shot", pane, theme);
    } catch (e) {
      console.error("FAILED", pane, theme, String(e).split("\n")[0]);
    }
  }
}
await browser.close();
console.log("done →", OUT);
