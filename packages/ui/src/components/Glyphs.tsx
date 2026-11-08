/** Status glyphs. Status changes the shape, not only the color (brand README rule 3). */

type GlyphProps = { size?: number; title?: string };

function a11y(title?: string) {
  return title ? { role: "img", "aria-label": title } : { "aria-hidden": true as const };
}

/** Amber triangle: needs you now. */
export function ActGlyph({ size = 14, title }: GlyphProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" {...a11y(title)}>
      <path d="M12 3 L22 20 L2 20 Z" fill="var(--ww-act)" stroke="var(--ww-act-edge)" strokeWidth={1.5} strokeLinejoin="round" />
    </svg>
  );
}

/** Filled sea-glass circle: all clear. */
export function OkGlyph({ size = 14, title }: GlyphProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" {...a11y(title)}>
      <circle cx="12" cy="12" r="9" fill="var(--ww-ok)" />
    </svg>
  );
}

/** Muted clock: an update is waiting for its slot in the plan, or is held. */
export function WaitGlyph({ size = 16, title }: GlyphProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="var(--ww-text-muted)" strokeWidth={2.2} strokeLinecap="round" {...a11y(title)}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 7 V12 L15 14" />
    </svg>
  );
}

/** Dashed circle: not checked against the update feed, so no claim either way. */
export function UncheckedGlyph({ size = 16, title }: GlyphProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="var(--ww-text-muted)" strokeWidth={2} strokeDasharray="3 3" {...a11y(title)}>
      <circle cx="12" cy="12" r="9" />
    </svg>
  );
}

/** Check mark: current. */
export function CheckGlyph({ size = 16, title }: GlyphProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="var(--ww-ok)"
      strokeWidth={3}
      strokeLinecap="round"
      strokeLinejoin="round"
      {...a11y(title)}
    >
      <path d="M5 12l5 5 9-10" />
    </svg>
  );
}

/** Phone outline, for "Update on the iPhone". */
export function PhoneGlyph({ size = 14, title }: GlyphProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" {...a11y(title)}>
      <rect x="7" y="2" width="10" height="20" rx="2.5" fill="none" stroke="currentColor" strokeWidth="2" />
      <path d="M11 18h2" stroke="currentColor" strokeWidth="2" />
    </svg>
  );
}

/** Down chevron for dropdowns. Decorative. */
export function ChevronGlyph({ size = 12 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 12 12" aria-hidden="true" className="pointer-events-none">
      <path d="M2.5 4.5 6 8l3.5-3.5" fill="none" stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
