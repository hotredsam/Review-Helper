// QA-only: screenshot the design gallery (served by `npm run dev` at /gallery.html)
// in a couple of states to verify the control bar + theme switching render.
import { chromium } from "playwright";

const b = await chromium.launch();
const p = await b.newPage({ viewport: { width: 1280, height: 920 }, deviceScaleFactor: 2 });
await p.goto("http://localhost:1420/gallery.html", { waitUntil: "domcontentloaded" });
await p.waitForTimeout(1500);
await p.screenshot({ path: "docs/ui-shots/gallery-overview.png" });
console.log("shot gallery-overview");

await p.click('button:has-text("Learn · Study")').catch((e) => console.error("learn click:", String(e).split("\n")[0]));
await p.click('button:has-text("Forest")').catch((e) => console.error("forest click:", String(e).split("\n")[0]));
await p.waitForTimeout(1500);
await p.screenshot({ path: "docs/ui-shots/gallery-learn-forest.png" });
console.log("shot gallery-learn-forest");

await b.close();
