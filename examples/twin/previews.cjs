#!/usr/bin/env node
// Takes the pictures in examples/twin/previews: each twin page and twin block, served by
// examples/serve.py, once its models are drawn. The Twins index and the Blocks page show
// these instead of live 3D views, since a browser holds only so many.
//
//   examples/build.sh && python3 examples/serve.py &
//   npm ci --prefix tools/playwright && npx --prefix tools/playwright playwright-core install firefox
//   node examples/twin/previews.cjs [http://localhost:8000]
const path = require("path");
const { firefox } = require(path.join(__dirname, "../../tools/playwright/node_modules/playwright-core"));
const base = process.argv[2] ?? "http://localhost:8000";
const out = path.join(__dirname, "previews");
const pages = ["world", "machine-data", "colour-by-value", "paths", "popups", "formats", "live-data", "editing", "big-world"].map((p) => [p, `twin/${p}.html`, 1280, 800]);
const blocks = ["twin-01", "twin-02", "twin-03", "twin-04"].map((b) => [b, `blocks/${b}.html`, 1440, 900]);

(async () => {
  const browser = await firefox.launch();
  for (const [name, url, width, height] of [...pages, ...blocks]) {
    const page = await browser.newPage({ viewport: { width, height } });
    await page.goto(`${base}/${url}`);
    const twin = page.locator("[data-i-x=ui-twin]").first();
    await twin.scrollIntoViewIfNeeded();
    await page.waitForSelector("[data-i-x=ui-twin][data-ready]", { timeout: 60000 });
    // Live values in, things moved along their paths.
    await page.waitForTimeout(2500);
    // A page's view and what's around it; a block as it opens.
    const clip = url.startsWith("twin/") ? await twin.evaluate((e) => {
      const r = e.getBoundingClientRect();
      return { x: Math.max(0, r.x - 4), y: Math.max(0, r.y - 4), width: Math.min(innerWidth, r.width + 8), height: Math.min(r.width * 0.625, r.height + 8) };
    }) : undefined;
    const file = path.join(out, `${name}.jpg`);
    await page.screenshot({ path: file, type: "jpeg", quality: 72, clip, scale: "css" });
    console.log(file);
    await page.close();
  }
  await browser.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
