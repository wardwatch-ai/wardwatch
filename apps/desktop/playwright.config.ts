import { defineConfig } from "@playwright/test";

// Browser tests of the desktop UI, run against the production Vite build with
// the sample scenario (no Tauri core). See e2e/visual/README.md.
const port = 4173;

export default defineConfig({
  testDir: "e2e",
  // Screenshots are only compared on Linux in CI, where fonts and rendering are pinned.
  snapshotPathTemplate: "{testDir}/visual/approved/{arg}{ext}",
  outputDir: "e2e/.results",
  // Baselines change only through e2e/visual/approve.ts, never by a test run.
  updateSnapshots: "none",
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  reporter: "list",
  expect: {
    // Same rule as review.spec.ts (sameImage in cases.ts): no pixel may change.
    toMatchSnapshot: { maxDiffPixels: 0, threshold: 0.1 },
  },
  use: {
    baseURL: `http://localhost:${String(port)}`,
    locale: "en-US",
    timezoneId: "UTC",
    deviceScaleFactor: 1,
  },
  projects: [
    { name: "visual", testMatch: "visual/visual.spec.ts" },
    { name: "visual-review", testMatch: "visual/review.spec.ts" },
    { name: "visual-mocks", testMatch: "visual/mocks.spec.ts" },
  ],
  webServer: {
    command: `pnpm exec vite build && pnpm exec vite preview --port ${String(port)} --strictPort`,
    port,
    reuseExistingServer: !process.env.CI,
  },
});
