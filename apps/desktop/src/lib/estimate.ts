// Estimates, on the device, how many points fixing pending updates now would add.
// The server only sends the sums behind your subscores (GET /v1/score `sums`);
// what is unpatched never leaves the device. This mirrors score algorithm v1 in
// wardwatch-ai/backend accounts/src/scoring/algorithm.ts and returns null for any
// other algorithm version.
import type { FeedSeverity, ScoreSums } from "./model";
import type { Program } from "./programs";

type Severity = "exploited" | "serious" | "moderate" | "minor";
const TARGET_DAYS: Record<Severity, number> = { exploited: 1, serious: 7, moderate: 14, minor: 30 };
const SEVERITY_WEIGHT: Record<Severity, number> = {
  exploited: 4,
  serious: 3,
  moderate: 2,
  minor: 1,
};

export interface PendingFix {
  exploited: boolean;
  severity: FeedSeverity | null;
  /** Browsers and the OS are the most popular programs; the app has no decile for the rest. */
  popular: boolean;
}

function scoreSeverity(p: PendingFix): Severity {
  if (p.exploited) return "exploited";
  if (p.severity === "critical" || p.severity === "high") return "serious";
  if (p.severity === "low") return "minor";
  // Medium, unknown or unrated: moderate, as the nudge engine treats unknown
  // severity (crates/wardwatch-nudge), never assumed minor.
  return "moderate";
}

function score(w: ScoreSums): number {
  const upToDate = Math.round((100 * w.in_time) / w.total);
  const fixSpeed = Math.round((100 * w.speed) / w.total);
  return Math.round((upToDate + fixSpeed) / 2);
}

function withFixes(w: ScoreSums, fixes: PendingFix[], days: number): ScoreSums {
  let { total, in_time, speed } = w;
  for (const f of fixes) {
    const sev = scoreSeverity(f);
    const weight = SEVERITY_WEIGHT[sev] * (0.5 + (f.popular ? 10 : 5) / 20);
    const target = TARGET_DAYS[sev];
    total += weight;
    if (days <= target) in_time += weight;
    speed += weight * Math.exp(-days / (2 * target));
  }
  return { total, in_time, speed };
}

/**
 * Points gained by fixing `fixes` today rather than `laterDays` from now, counting
 * from today (the notice date isn't known on the device). Null when it can't be
 * estimated; 0 is a real answer.
 */
export function pointsForFixingNow(
  sums: ScoreSums | undefined,
  algorithm: number,
  fixes: PendingFix[],
  laterDays: number,
): number | null {
  if (!sums || algorithm !== 1 || fixes.length === 0) return null;
  const gain = score(withFixes(sums, fixes, 0)) - score(withFixes(sums, fixes, laterDays));
  return Math.max(0, gain);
}

/** Whole days until the Sunday tune-up (0 on a Sunday). */
export function daysToSunday(now: Date): number {
  return (7 - now.getDay()) % 7;
}

const rank: Record<FeedSeverity, number> = { critical: 4, high: 3, medium: 2, unknown: 1, low: 0 };

/**
 * The pending update as the scorer would see it, or null when it fixes no known
 * hole (scores only count security fixes). Severity is its worst hole's.
 */
export function pendingFix(p: Program): PendingFix | null {
  if (p.vulns.length === 0) return null;
  const worst = p.vulns
    .map((v) => v.severity ?? "unknown")
    .reduce((a, b) => (rank[b] > rank[a] ? b : a));
  return { exploited: p.state === "urgent", severity: worst, popular: p.kind !== "apps" };
}

/** pendingFix for each program that has one. */
export function pendingFixes(programs: Program[]): PendingFix[] {
  return programs.flatMap((p) => pendingFix(p) ?? []);
}
