// Builds the review set in out/: for each screen and theme, the app (plus the
// mock and a diff of the two when the screen has a mock), and <id>.json saying whether the app changed since its last verdict.
// Matt marks each one "ok" or "fix"; approve.ts then records his verdicts.
import { test } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import pixelmatch from "pixelmatch";
import { PNG } from "pngjs";
import {
  captureApp,
  cases,
  dir,
  mockBytes,
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
  // Start clean, so no file from an earlier run sits beside this one's.
  rmSync(outDir, { recursive: true, force: true });
  mkdirSync(outDir, { recursive: true });
});

for (const c of cases) {
  for (const theme of themes) {
    const id = `${c.id}-${theme}`;
    test(id, async ({ page }) => {
      const app = await captureApp(page, c, theme);
      const a = PNG.sync.read(app);
      writeFileSync(join(outDir, `${id}.app.png`), app);
      // With a mock: the mock and a diff against it. Without one, Matt reviews the app alone.
      const mock = mockBytes(c, theme);
      let mismatch: number | null = null;
      let mockSize: number[] | null = null;
      if (mock) {
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
        writeFileSync(join(outDir, `${id}.mock.png`), mock);
        writeFileSync(join(outDir, `${id}.diff.png`), PNG.sync.write(diff));
        mismatch = Number(((100 * changed) / (width * height)).toFixed(2));
        mockSize = [m.width, m.height];
      }
      const mockSha = mock ? sha256(mock) : null;

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
        mockFile: c.mock ?? null,
        size: { app: [a.width, a.height], mock: mockSize },
        mismatch,
        app: sha256(app),
        mock: mockSha,
        // Unchanged: the app and the mock both look like what Matt last saw.
        unchanged:
          approval !== undefined &&
          approval.mock === mockSha &&
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
