// Small text helpers the screens share.
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
