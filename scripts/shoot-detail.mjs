// QA-only: load a preview pane, optionally click an element, then screenshot —
// for capturing detail/expanded states.  Requires `npm run dev` on :1420.
//   node scripts/shoot-detail.mjs <pane> <clickText> <outName> <theme>
import { chromium } from "playwright";

const pane = process.argv[2] || "understand";
const clickText = process.argv[3] || "";
const outName = process.argv[4] || "detail";
const theme = process.argv[5] || "light";

const b = await chromium.launch();
const p = await b.newPage({ viewport: { width: 1200, height: 1000 }, deviceScaleFactor: 2 });
await p.goto(`http://localhost:1420/preview.html?pane=${pane}&theme=${theme}`, { waitUntil: "domcontentloaded" });
await p.waitForTimeout(1200);
if (clickText) {
  await p.click(`button:has-text("${clickText}")`).catch((e) => console.error("click failed:", String(e).split("\n")[0]));
  await p.waitForTimeout(1500);
}
await p.screenshot({ path: `docs/ui-shots/${outName}.png` });
console.log("shot", outName);
await b.close();
