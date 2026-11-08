import type { ButtonHTMLAttributes } from "react";
import { cx } from "./cx";

type Option<T extends string> = { value: T; label: string };

/** One-of-several switch, e.g. Groups' Last 30 days / This month / Since joining. */
export function SegmentedControl<T extends string>({
  options,
  value,
  onChange,
  label,
}: {
  options: readonly Option<T>[];
  value: T;
  onChange: (value: T) => void;
  label: string;
}) {
  return (
    <div className="ww-seg" role="group" aria-label={label}>
      {options.map((o) => (
        <button key={o.value} type="button" aria-pressed={o.value === value} onClick={() => onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

/** Toggle chip, e.g. Update plan presets. `on` is false when the grid matches no preset. */
export function Chip({ on, className, ...rest }: ButtonHTMLAttributes<HTMLButtonElement> & { on: boolean }) {
  return <button type="button" {...rest} aria-pressed={on} className={cx("ww-chip", className)} />;
}
