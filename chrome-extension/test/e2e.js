// Chạy: cd chrome-extension/test && npm install && xvfb-run -a npm test
const path = require("path");
const { chromium } = require("playwright");

(async () => {
  const root = path.resolve(__dirname, "..");
  const dist = path.join(root, "dist");
  const context = await chromium.launchPersistentContext(
    path.join(root, ".test-profile"),
    {
      headless: false,
      args: [
        `--disable-extensions-except=${dist}`,
        `--load-extension=${dist}`,
        "--no-first-run",
        "--no-default-browser-check",
      ],
    },
  );
  try {
    let workers = context.serviceWorkers();
    if (!workers.length) {
      workers = [await context.waitForEvent("serviceworker", { timeout: 15000 })];
    }
    const id = new URL(workers[0].url()).host;
    const page = await context.newPage();
    await page.goto(`chrome-extension://${id}/popup.html`);
    for (const [input, expected] of [
      ["tieengs", "tiếng"],
      ["tiếng", "tiếng"],
      ["data", "data"],
      ["hello", "hello"],
      ["Vieejt", "Việt"],
    ]) {
      await page.fill("#input", input);
      await page.click("#normalize");
      await page.waitForTimeout(300);
      const output = await page.inputValue("#output");
      if (output !== expected) {
        throw new Error(`${input}: expected ${expected}, got ${output}`);
      }
    }
    console.log("Chrome extension E2E: PASS");
  } finally {
    await context.close();
  }
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
