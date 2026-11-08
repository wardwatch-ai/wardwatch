// Small pieces several screens share.
import { ActGlyph, Button, CheckGlyph } from "@wardwatch/ui";
import type { ReactNode } from "react";
import { Link, useLocation, useNavigate } from "react-router";
import { type Program, fixPath } from "../lib/programs";
import { tabs } from "./routes";
import { type From, useOpen } from "./useOpen";

const screenNames: Record<string, string> = {
  ...Object.fromEntries(tabs.map((t) => [t.path, t.label])),
  "/fix/tune-up": "Sunday tune-up",
};

/**
 * Top of the fix and tune-up screens: back to the screen that opened it. Opened
 * from the menu bar (no in-app history), it goes Home.
 */
export function BackBar({ children }: { children?: ReactNode }) {
  const navigate = useNavigate();
  const from = (useLocation().state as From | null)?.from;
  const name = from ? screenNames[from] : undefined;
  return (
    <div className="flex items-center gap-3">
      {name ? (
        <button type="button" className="ww-btn ww-btn-quiet" onClick={() => void navigate(-1)}>
          <span aria-hidden="true">←</span> Back to {name}
        </button>
      ) : (
        <Link to="/" className="ww-btn ww-btn-quiet">
          <span aria-hidden="true">←</span> Home
        </Link>
      )}
      {children}
    </div>
  );
}

/**
 * Update button for a program on this device. Amber only when it's being
 * exploited; otherwise quiet with the severity meter. Both open the fix screen,
 * which does the update.
 */
export function UpdateButton({
  program,
  children = "Update now",
  className,
}: {
  program: Program;
  children?: ReactNode;
  className?: string;
}) {
  const open = useOpen();
  const go = () => {
    open(fixPath(program));
  };
  return program.state === "urgent" ? (
    <Button variant="act" className={className} onClick={go}>
      {children}
    </Button>
  ) : (
    <Button variant="quiet" severity={program.severity} className={className} onClick={go}>
      {children}
    </Button>
  );
}

export function Loading() {
  return <p className="text-text-muted">Loading</p>;
}

export function Failed({ error }: { error: Error }) {
  return <p role="alert">Something went wrong: {error.message}</p>;
}

/** Status mark for a System / Browsers / Apps line. Shape, not only color, shows the state. */
export function KindMark({ mark }: { mark: "urgent" | "current" | "none" }) {
  if (mark === "urgent") return <ActGlyph size={16} />;
  if (mark === "current") return <CheckGlyph />;
  return <span className="w-4 flex-none" />;
}

/**
 * A dropdown whose visible label is ordinary page text. The native select sits on top,
 * transparent, so the keyboard, screen readers and the OS menu work as usual; Chromium
 * paints native select text slightly differently between machines, which made the
 * design check flaky.
 */
export function Select<T extends string>({
  label,
  value,
  options,
  onChange,
  className = "",
}: {
  label: string;
  value: T;
  options: readonly { value: T; label: string }[];
  onChange: (value: T) => void;
  className?: string;
}) {
  const shown = options.find((o) => o.value === value)?.label ?? value;
  return (
    <div
      className={`relative flex min-h-10 items-center rounded-[8px] border border-edge bg-surface pr-7 pl-3 text-sm text-text has-[:focus-visible]:outline-3 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-text ${className}`}
    >
      <span aria-hidden className="truncate">
        {shown}
      </span>
      <svg
        aria-hidden
        width="12"
        height="12"
        viewBox="0 0 12 12"
        className="pointer-events-none absolute right-2.5"
      >
        <path
          d="M2.5 4.5 6 8l3.5-3.5"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
      <select
        aria-label={label}
        value={value}
        onChange={(e) => {
          onChange(e.target.value as T);
        }}
        className="absolute inset-0 size-full cursor-pointer opacity-0"
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </div>
  );
}
