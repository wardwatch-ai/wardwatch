// Builds the review set in out/: for each screen and theme, the mock, the app,
// a diff of the two, and <id>.json saying whether the app changed since its last verdict.
// Matt marks each one "ok" or "fix"; approve.ts then records his verdicts.
import { test } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import pixelmatch from "pixelmatch";
import { PNG } from "pngjs";
import {
  captureApp,
  cases,
  dir,
  mockPng,
  outDir,
  readApprovals,
  sameImage,
  sha256,
  themes,
} from "./cases.ts";

/** Pads an image to width x height with its own top-left pixel (the page background). */
function pad(img: PNG, width: number, height: number): PNG {
  if (img.width === width && img.height === height) return img;
  const out = new PNG({ width, height });
  for (let i = 0; i < width * height; i++) img.data.copy(out.data, i * 4, 0, 4);
  PNG.bitblt(img, out, 0, 0, img.width, img.height, 0, 0);
  return out;
}

const approvals = readApprovals();
// The commit the screens were rendered from, so approve.ts can refuse a stale review.
// CI checks out a merge commit, so it passes the PR head as VISUAL_COMMIT.
const commit =
  process.env.VISUAL_COMMIT ||
  execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();

test.beforeAll(() => {
  mkdirSync(outDir, { recursive: true });
});

for (const c of cases) {
  for (const theme of themes) {
    const id = `${c.id}-${theme}`;
    test(id, async ({ page }) => {
      const app = await captureApp(page, c, theme);
      const mock = readFileSync(mockPng(c.id, theme));
      const a = PNG.sync.read(app);
      const m = PNG.sync.read(mock);
      const width = Math.max(a.width, m.width);
      const height = Math.max(a.height, m.height);
      const diff = new PNG({ width, height });
      const changed = pixelmatch(
        pad(m, width, height).data,
        pad(a, width, height).data,
        diff.data,
        width,
        height,
        { threshold: 0.1, alpha: 0.3 },
      );
      writeFileSync(join(outDir, `${id}.app.png`), app);
      writeFileSync(join(outDir, `${id}.mock.png`), mock);
      writeFileSync(join(outDir, `${id}.diff.png`), PNG.sync.write(diff));

      const approval = approvals[id];
      const approvedFile = join(dir, "approved", `${id}.png`);
      const approved = existsSync(approvedFile)
        ? PNG.sync.read(readFileSync(approvedFile))
        : undefined;
      const row = {
        id,
        commit,
        case: c.id,
        theme,
        title: c.title,
        mockFile: c.mock,
        size: { app: [a.width, a.height], mock: [m.width, m.height] },
        mismatch: Number(((100 * changed) / (width * height)).toFixed(2)),
        app: sha256(app),
        mock: sha256(mock),
        // Unchanged: the app and the mock both look like what Matt last saw.
        unchanged:
          approval?.mock === sha256(mock) &&
          approved?.width === a.width &&
          approved.height === a.height &&
          pixelmatch(approved.data, a.data, null, a.width, a.height, sameImage) === 0,
        last: approval ?? null,
      };
      // One file per screen: a failed test restarts the worker, so nothing is
      // collected in memory across tests.
      writeFileSync(join(outDir, `${id}.json`), JSON.stringify(row, null, 2) + "\n");
    });
  }
}
