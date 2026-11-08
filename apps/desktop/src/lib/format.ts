// Small text helpers the screens share.
import type { AppInfo } from "./ipc";
import type { Program } from "./programs";

/** "142.0.7444.59 → 142.0.7444.162" */
export function versionStep(p: Program) {
  return p.latest ? `${p.version} → ${p.latest}` : p.version;
}

/** Highest severity among programs, for a batch button's meter. */
export function worstSeverity(programs: Program[]) {
  return programs.reduce<1 | 2 | 3>((s, p) => (p.severity > s ? p.severity : s), 1);
}

/** Ordinal for group ranks: 1st, 2nd, 3rd, 4th. */
export function ordinal(n: number) {
  const tens = n % 100;
  const suffix = tens >= 11 && tens <= 13 ? "th" : (["th", "st", "nd", "rd"][n % 10] ?? "th");
  return `${String(n)}${suffix}`;
}

/**
 * The version as people read it. Nightlies are versioned "next release, then
 * days since 2026-01-01" (0.2.0-312), which shows as "0.2.0 nightly 312".
 */
export function releaseLabel(info: Pick<AppInfo, "version" | "channel">) {
  const m = /^(?<base>\d+\.\d+\.\d+)-(?<n>\d+)$/.exec(info.version)?.groups;
  if (info.channel === "nightly" && m?.base && m.n) return `${m.base} nightly ${m.n}`;
  return info.version;
}

/** "2026-11-09" → "Nov 9, 2026" (the date the build was made, in UTC). */
export function buildDate(day: string) {
  const date = new Date(`${day}T00:00:00Z`);
  if (Number.isNaN(date.getTime())) return day;
  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
    timeZone: "UTC",
  });
}
