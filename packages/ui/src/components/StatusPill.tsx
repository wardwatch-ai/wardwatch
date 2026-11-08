import type { ReactNode } from "react";
import { OkGlyph, PhoneGlyph } from "./Glyphs";

/** ok: green "Up to date" label. remote: dashed "Update on the iPhone" label for another device's fix. */
export function StatusPill({ tone, children }: { tone: "ok" | "remote"; children: ReactNode }) {
  return (
    <span className={`ww-pill ww-pill-${tone}`}>
      {tone === "ok" ? <OkGlyph /> : <PhoneGlyph />}
      {children}
    </span>
  );
}
