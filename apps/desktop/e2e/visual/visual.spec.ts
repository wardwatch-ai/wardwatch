// The gate CI runs: every screen must look exactly like the version Matt last
// reviewed against its mock. A screen that changed, a mock that changed, or a
// screen nobody reviewed yet fails, and `pnpm visual:review` shows why.
import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";
import { captureApp, cases, mockPng, readApprovals, sha256, themes } from "./cases.ts";

const approvals = readApprovals();

for (const c of cases) {
  for (const theme of themes) {
    const id = `${c.id}-${theme}`;
    test(id, async ({ page }) => {
      const approval = approvals[id];
      expect(approval, `${id} has not been reviewed yet; run pnpm visual:review`).toBeDefined();
      expect(
        approval?.mock,
        `the ${c.title} mock changed since it was reviewed; run pnpm visual:review`,
      ).toBe(sha256(readFileSync(mockPng(c.id, theme))));

      const png = await captureApp(page, c, theme);
      expect(png, `${id} changed since Matt reviewed it; run pnpm visual:review`).toMatchSnapshot(
        `${id}.png`,
      );
    });
  }
}
