// Small pieces several screens share.
import { ActGlyph, Button, CheckGlyph } from "@wardwatch/ui";
import type { ReactNode } from "react";
import { useNavigate } from "react-router";
import { type Program, fixPath } from "../lib/programs";

/**
 * Update button for a program on this device. Amber only when it's being
 * exploited; otherwise quiet with the severity meter. Both open the fix screen,
 * which does the update.
 */
export function UpdateButton({
  program,
  children = "Update now",
}: {
  program: Program;
  children?: ReactNode;
}) {
  const navigate = useNavigate();
  const go = () => void navigate(fixPath(program));
  return program.state === "urgent" ? (
    <Button variant="act" onClick={go}>
      {children}
    </Button>
  ) : (
    <Button variant="quiet" severity={program.severity} onClick={go}>
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
