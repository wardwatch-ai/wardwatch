import type { AnchorHTMLAttributes, ButtonHTMLAttributes, ReactNode } from "react";
import { cx } from "./cx";
import { SeverityMeter, severityLabel, type Severity } from "./SeverityMeter";

/**
 * act:   amber act-now button. Only for a hole being exploited right now (brand README rule 6).
 * brand: ordinary primary action (Invite, Share score, Done).
 * quiet: outline button. Pass `severity` on update buttons that are not urgent.
 */
type Variant =
  | { variant: "act"; severity?: never }
  | { variant: "brand"; severity?: never }
  | { variant: "quiet"; severity?: Severity };

type Common = Variant & { children: ReactNode; className?: string };
type AsButton = Common & Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className" | "children"> & { href?: never };
type AsLink = Common & Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "className" | "children"> & { href: string };
export type ButtonProps = AsButton | AsLink;

export function Button(props: ButtonProps) {
  const { variant, severity, children, className, ...rest } = props;
  const classes = cx("ww-btn", `ww-btn-${variant}`, className);
  const content = (
    <>
      {severity && <SeverityMeter level={severity} />}
      {children}
      {severity && <span className="ww-sr-only">{`, ${severityLabel[severity].toLowerCase()}`}</span>}
    </>
  );
  if (rest.href !== undefined) {
    return (
      <a {...(rest as AnchorHTMLAttributes<HTMLAnchorElement>)} className={classes}>
        {content}
      </a>
    );
  }
  const { type = "button", ...buttonRest } = rest as ButtonHTMLAttributes<HTMLButtonElement>;
  return (
    <button {...buttonRest} type={type} className={classes}>
      {content}
    </button>
  );
}
