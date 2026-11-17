#!/usr/bin/env node
// Stands in for `chrome --headless --dump-dom URL`, so the browser tests run in other
// engines unchanged: it opens URL in the Playwright engine named by $BROWSER (webkit or
// firefox), waits for the driver's #result, and prints the DOM.
//
//   npm ci --prefix tools/playwright && npx --prefix tools/playwright playwright-core install webkit firefox
//   BROWSER=webkit CHROME=$PWD/tools/playwright/dump-dom.js cargo test --workspace --test browser -- --ignored
const playwright = require("playwright-core");

const args = process.argv.slice(2);
if (args.includes("--version")) {
  console.log("dump-dom (playwright-core)");
  process.exit(0);
}
const flag = (name, fallback) => (args.find((a) => a.startsWith(`--${name}=`)) ?? `=${fallback}`).split("=")[1];
const url = args.find((a) => !a.startsWith("--"));
const [width, height] = flag("window-size", "1000,900").split(",").map(Number);
const budget = Number(flag("virtual-time-budget", 10000));
const engine = process.env.BROWSER ?? "webkit";
if (!["webkit", "firefox", "chromium"].includes(engine)) {
  console.error(`BROWSER must be webkit, firefox or chromium, not ${engine}`);
  process.exit(2);
}

(async () => {
  const browser = await playwright[engine].launch();
  const page = await browser.newPage({
    viewport: { width, height },
    // The tests expect System to mean light, as `--blink-settings=preferredColorScheme=1` makes it in Chrome.
    colorScheme: "light",
    reducedMotion: args.includes("--force-prefers-reduced-motion") ? "reduce" : "no-preference",
  });
  page.on("pageerror", (e) => console.error(`pageerror: ${e.message}`));
  await page.goto(url);
  // Chrome's virtual time skips waits; here they take real time, so allow the budget and some slack.
  // A driver that never writes #result still gets its DOM printed, and the test reports it.
  await page.waitForSelector("#result", { state: "attached", timeout: budget + 15000 }).catch(() => {});
  process.stdout.write(await page.content());
  await browser.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
