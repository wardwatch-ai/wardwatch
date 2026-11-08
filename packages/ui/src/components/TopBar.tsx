import type { AnchorHTMLAttributes, ButtonHTMLAttributes, ReactNode } from "react";
import { cx } from "./cx";
import { Lighthouse } from "./Lighthouse";

/** Window top bar: lighthouse and name, the main tabs, and the account pill. */
export function TopBar({ beams, nav, account }: { beams: boolean; nav: ReactNode; account?: ReactNode }) {
  return (
    <header className="ww-topbar">
      <div className="ww-topbar-brand">
        <Lighthouse beams={beams} />
        <span className="ww-display">Wardwatch</span>
      </div>
      <nav aria-label="Main" className="ww-topbar-nav">
        {nav}
      </nav>
      {account}
    </header>
  );
}

/**
 * A main tab. With a router, put className="ww-tab" and aria-current="page" on its own
 * link component instead (React Router's NavLink sets aria-current for you).
 */
export function NavTab({ current, className, ...rest }: AnchorHTMLAttributes<HTMLAnchorElement> & { current?: boolean }) {
  return <a {...rest} className={cx("ww-tab", className)} aria-current={current ? "page" : rest["aria-current"]} />;
}

type AccountPillProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> & {
  name: string;
  score: number;
};

/** Account pill (avatar, name, score). Its menu, listing the account's devices, belongs to the app. */
export function AccountPill({ name, score, className, ...rest }: AccountPillProps) {
  return (
    <button type="button" aria-haspopup="menu" {...rest} className={cx("ww-account", className)}>
      <span className="ww-avatar" aria-hidden="true">
        {name.trim().charAt(0).toUpperCase()}
      </span>
      {name}
      <span className="ww-display" style={{ color: "var(--ww-ok)" }} aria-hidden="true">
        {score}
      </span>
      <span className="ww-sr-only">{`, score ${score}`}</span>
    </button>
  );
}
