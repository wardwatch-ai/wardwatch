import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "./cx";

/**
 * default: surface with a hairline border.
 * urgent:  amber tint, only for something being exploited right now.
 * ok:      sea-glass tint (score strip, all clear).
 * inset:   page-colored panel inside a card (subscores, "You have → Newest").
 * remote:  dashed outline, for a fix that has to happen on another device. Give it no button.
 */
export type CardTone = "default" | "urgent" | "ok" | "inset" | "remote";

type CardProps = HTMLAttributes<HTMLElement> & {
  tone?: CardTone;
  as?: "section" | "div" | "article" | "aside";
  children: ReactNode;
};

export function Card({ tone = "default", as: Tag = "section", className, children, ...rest }: CardProps) {
  return (
    <Tag {...rest} className={cx("ww-card", `ww-card-${tone}`, className)}>
      {children}
    </Tag>
  );
}

/** Small uppercase label above a section ("Raise your score", "By device"). */
export function Eyebrow({ className, ...rest }: HTMLAttributes<HTMLDivElement>) {
  return <div {...rest} className={cx("ww-eyebrow", className)} />;
}
