// Every screen Matt approved on the final design canvas, paired with the app
// screen that implements it. Each case runs in light and dark.
// Canvas: https://claude.ai/artifact/Nv9Qdr7hGLdMpBoCFUuXvo
import type { Page } from "@playwright/test";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";

export interface VisualCase {
  id: string;
  title: string;
  /** The approved artboard in mocks/src. */
  mock: string;
  /** App page and hash route to open. */
  url: string;
  viewport: { width: number; height: number };
  /** The app's clock, when the screen needs another day than fixedNow. */
  now?: Date;
}

const main = { width: 1280, height: 900 };

export const cases: VisualCase[] = [
  { id: "home", title: "Home", mock: "Harbor.dc.html", url: "/#/", viewport: main },
  {
    id: "tray",
    title: "Menu bar popover",
    mock: "Tray.dc.html",
    url: "/tray.html",
    viewport: { width: 400, height: 740 },
  },
  {
    id: "fix",
    title: "Fix screen (Chrome)",
    mock: "Fix.dc.html",
    url: "/#/fix/mac_app/com.google.Chrome",
    viewport: main,
  },
  {
    id: "score",
    title: "Score",
    mock: "Score6.dc.html",
    url: "/#/score",
    viewport: main,
    // A Wednesday, as in the approved design. fixedNow is a Sunday, when the
    // tune-up is today and doing it early gains nothing, so no "+N" shows.
    now: new Date("2026-11-11T12:00:00Z"),
  },
  { id: "groups", title: "Groups", mock: "Groups4.dc.html", url: "/#/groups", viewport: main },
  { id: "apps", title: "Apps", mock: "Apps3.dc.html", url: "/#/apps", viewport: main },
  { id: "plan", title: "Update plan", mock: "Plan4.dc.html", url: "/#/plan", viewport: main },
];

export const themes = ["light", "dark"] as const;
export type Theme = (typeof themes)[number];

export const dir = import.meta.dirname;
export const mockPng = (id: string, theme: Theme) => join(dir, "mocks", `${id}-${theme}.png`);
export const outDir = join(dir, "out");
export const approvalsFile = join(dir, "approvals.json");

/**
 * Two renders count as the same screen only when no pixel differs beyond
 * pixelmatch's colour threshold. Renders are byte-identical on the pinned
 * platform, so any real change, even one digit, needs a new review.
 */
export const sameImage = { threshold: 0.1 };

export const sha256 = (data: Buffer) => createHash("sha256").update(data).digest("hex");

export interface Approval {
  verdict: "ok" | "fix";
  note: string;
  /** sha256 of the mock PNG and the app PNG Matt looked at. */
  mock: string;
  app: string;
  by: string;
  at: string;
  review: string;
}

export type Approvals = Partial<Record<string, Approval>>;

export function readApprovals(): Approvals {
  return JSON.parse(readFileSync(approvalsFile, "utf8")) as Approvals;
}

// Sample data shows times relative to now, so every run uses the same clock.
export const fixedNow = new Date("2026-11-08T12:00:00Z");

/**
 * Opens one app screen with the browser sample scenario and returns a full-page
 * PNG. The gate and the review both capture through here, so the picture Matt
 * approves is the picture CI compares against.
 */
export async function captureApp(page: Page, c: VisualCase, theme: Theme): Promise<Buffer> {
  await page.clock.setFixedTime(c.now ?? fixedNow);
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.setViewportSize(c.viewport);
  await page.goto(c.url, { waitUntil: "networkidle" });
  await page.evaluate(() => document.fonts.ready);
  return page.screenshot({ fullPage: true, animations: "disabled", caret: "hide" });
}
