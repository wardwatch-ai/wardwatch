import { cx } from "./cx";

export const bands = [
  { name: "At risk", min: 0, label: "At risk · below 60" },
  { name: "Fair", min: 60, label: "Fair · 60" },
  { name: "Good", min: 75, label: "Good · 75" },
  { name: "Excellent", min: 90, label: "Excellent · 90" },
] as const;

export type BandName = (typeof bands)[number]["name"];

const clamp = (n: number) => Math.min(100, Math.max(0, n));

/** The band a 0-100 score falls in. */
export function bandFor(score: number): BandName {
  const s = clamp(score);
  return [...bands].reverse().find((b) => s >= b.min)!.name;
}

/** Score band bar: four bands with a marker at the score. */
export function BandBar({ score }: { score: number }) {
  const s = clamp(score);
  const band = bandFor(s);
  return (
    <div className="ww-band" role="img" aria-label={`Score ${s}, ${band} band`}>
      <div className="ww-band-track" aria-hidden="true">
        <span />
        <span />
        <span />
        <span />
        <span className="ww-band-marker" style={{ left: `${s}%` }} />
      </div>
      <div className="ww-band-labels" aria-hidden="true">
        {bands.map((b) => (
          <span key={b.name} className={cx(b.name === band && "ww-band-current")}>
            {b.label}
          </span>
        ))}
      </div>
    </div>
  );
}

/** Thin 0-100 bar for a subscore ("How up to date you are"). */
export function SubscoreBar({ value, label }: { value: number; label: string }) {
  const v = clamp(value);
  return (
    <div className="ww-bar" role="meter" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={v}>
      <span style={{ width: `${v}%` }} />
    </div>
  );
}
