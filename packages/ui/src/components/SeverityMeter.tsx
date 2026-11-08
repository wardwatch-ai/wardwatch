import { cx } from "./cx";

/** 3 = serious, 2 = moderate, 1 = minor or no security fix. */
export type Severity = 1 | 2 | 3;

export const severityLabel: Record<Severity, string> = {
  3: "Serious",
  2: "Moderate",
  1: "Minor or no security fix",
};

/** The 3-bar severity meter. Visual only; pair it with text (Button adds it for you). */
export function SeverityMeter({ level, className }: { level: Severity; className?: string }) {
  return (
    <span className={cx("ww-meter", className)} aria-hidden="true" data-level={level}>
      {[1, 2, 3].map((n) => (
        <i key={n} className={n <= level ? "ww-meter-on" : undefined} />
      ))}
    </span>
  );
}
