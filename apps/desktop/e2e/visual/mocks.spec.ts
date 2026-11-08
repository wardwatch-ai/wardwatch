// Renders the approved artboards in mocks/src to mocks/<case>-<theme>.png.
// Only needed when the design canvas changes; the PNGs are committed.
//
// The artboards need the design canvas runtime, which is not ours to commit.
// Download it from the canvas (artifact-type/dc-runtime.js) and run:
//   DC_RUNTIME=/path/to/dc-runtime.js pnpm visual:mocks
import { expect, test } from "@playwright/test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { cases, dir, fixedNow, mockPng, themes } from "./cases.ts";

const runtime = process.env.DC_RUNTIME;
const origin = "http://mocks.test";
const fonts = join(dir, "../../../../packages/ui/src/fonts");

// The artboards ask Google Fonts for Inter and Manrope. Serve the brand files
// the app bundles instead, so both sides draw text with the same font files.
const fontCss = [
  ["Inter", 400, "inter-400"],
  ["Inter", 600, "inter-600"],
  ["Manrope", 800, "manrope-800"],
]
  .map(
    ([family, weight, file]) =>
      `@font-face{font-family:${String(family)};font-weight:${String(weight)};src:url(${origin}/fonts/${String(file)}.woff2) format("woff2")}`,
  )
  .join("\n");

test.skip(!runtime, "set DC_RUNTIME to the canvas runtime to render mocks");

for (const c of cases) {
  for (const theme of themes) {
    test(`${c.id}-${theme}`, async ({ page }) => {
      await page.route("**/*", async (route) => {
        const url = new URL(route.request().url());
        if (url.hostname === "fonts.googleapis.com") {
          return route.fulfill({ contentType: "text/css", body: fontCss });
        }
        if (url.origin !== origin) return route.abort();
        if (url.pathname === "/support.js") {
          return route.fulfill({
            contentType: "text/javascript",
            body: readFileSync(runtime ?? ""),
          });
        }
        if (url.pathname.startsWith("/fonts/")) {
          return route.fulfill({
            contentType: "font/woff2",
            body: readFileSync(join(fonts, url.pathname.slice("/fonts/".length))),
          });
        }
        let html = readFileSync(join(dir, "mocks/src", c.mock), "utf8");
        // Each artboard has a "dark" prop; flip its default for the dark render.
        if (theme === "dark") {
          const light = `"dark":{"editor":"boolean","default":false}`;
          if (!html.includes(light)) throw new Error(`${c.mock} has no dark prop to switch on`);
          html = html.replace(light, `"dark":{"editor":"boolean","default":true}`);
        }
        return route.fulfill({ contentType: "text/html", body: html });
      });
      await page.clock.setFixedTime(fixedNow);
      await page.setViewportSize(c.viewport);
      await page.goto(`${origin}/${c.mock}`, { waitUntil: "networkidle" });
      await expect(page.locator(".ww").first()).toBeVisible();
      await page.evaluate(() => document.fonts.ready);
      const png = await page.screenshot({ fullPage: true, animations: "disabled", caret: "hide" });
      writeFileSync(mockPng(c.id, theme), png);
    });
  }
}
