/**
 * The Wardwatch lighthouse (Old Barney). `beams` on means something needs the user now;
 * the shape changes, not only the color. Colors follow the active theme, except the white
 * lower tower, which is white in both themes like the brand kit's mark.svg and mark-on-dark.svg.
 */
export function Lighthouse({ beams, size = 30, title }: { beams: boolean; size?: number; title?: string }) {
  const label = title ?? (beams ? "Something needs you" : "All clear");
  return (
    <svg width={size} height={size} viewBox="0 0 64 64" role="img" aria-label={label} data-beams={beams}>
      {beams && (
        <>
          <path d="M28 16.5 L3 9 L3 24 Z" fill="var(--ww-act)" />
          <path d="M36 16.5 L61 9 L61 24 Z" fill="var(--ww-act)" />
        </>
      )}
      <path d="M26 13 L32 7.5 L38 13 Z" fill="var(--ww-tower)" />
      <rect x="27.5" y="13" width="9" height="7" fill={beams ? "var(--ww-act)" : "var(--ww-tower)"} />
      {beams && <rect x="31.4" y="13" width="1.2" height="7" fill="var(--ww-tower)" />}
      <rect x="24" y="20" width="16" height="3" rx="1" fill="var(--ww-tower)" />
      <path d="M26.5 23 L37.5 23 L39.2 40 L24.8 40 Z" fill="var(--ww-brand)" />
      <path d="M24.8 40 L39.2 40 L41 57 L23 57 Z" fill="#FFFFFF" stroke="var(--ww-act-edge)" strokeWidth="1.5" strokeLinejoin="round" />
      <rect x="21" y="57" width="22" height="3" rx="1" fill="var(--ww-tower)" />
    </svg>
  );
}
