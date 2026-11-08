import { describe, expect, it } from "vitest";
import { colors, type ColorToken, type ThemeName } from "../src/tokens";
import { contrast } from "./wcag";

// [foreground, background, minimum ratio]. Text needs 4.5:1, controls and graphics 3:1
// (brand README rule 4, design spec "Accessibility").
const TEXT = 4.5;
const GRAPHIC = 3;
const pairs: Array<[ColorToken, ColorToken, number]> = [
  // Body and secondary text on every surface it sits on
  ...(["bg", "surface", "sel", "act-tint", "ok-tint", "code"] as const).flatMap(
    (bg): Array<[ColorToken, ColorToken, number]> => [
      ["text", bg, TEXT],
      ["text-muted", bg, TEXT],
    ],
  ),
  ["link", "bg", TEXT],
  ["link", "surface", TEXT],
  ["ok", "bg", TEXT],
  ["ok", "surface", TEXT],
  ["ok", "ok-tint", TEXT],
  ["act-ink", "act-tint", TEXT],
  ["act-ink", "surface", TEXT],
  ["on-brand", "brand", TEXT],
  ["on-act", "act", TEXT],
  ["bg", "ok", TEXT], // avatar initial
  ["bg", "text", TEXT], // selected chip
  // Controls: quiet button and chip outlines, primary button fill
  ...(["bg", "surface", "act-tint", "sel", "ok-tint"] as const).map(
    (bg): [ColorToken, ColorToken, number] => ["edge", bg, GRAPHIC],
  ),
  ["brand", "bg", GRAPHIC],
  ["brand", "surface", GRAPHIC],
  // Graphics: severity meter (filled bar vs empty bar, empty bar vs button background),
  // the off lantern, the act glyph's edge on the urgent card
  ["text", "edge", GRAPHIC],
  ["edge", "surface", GRAPHIC],
  ["edge", "bg", GRAPHIC],
  ["tower", "surface", GRAPHIC],
];

describe.each(["light", "dark"] as ThemeName[])("%s theme contrast", (theme) => {
  const p = colors[theme];
  it.each(pairs)("%s on %s is at least %s:1", (fg, bg, min) => {
    expect(contrast(p[fg], p[bg])).toBeGreaterThanOrEqual(min);
  });

  it("score band marker stands out from every band (ring or fill at 3:1)", () => {
    for (const band of ["band-1", "band-2", "band-3", "band-4"] as const) {
      const best = Math.max(contrast(p.text, p[band]), contrast(p.surface, p[band]));
      expect(best, band).toBeGreaterThanOrEqual(GRAPHIC);
    }
  });
});

describe("act-now button (brand README rule 6)", () => {
  it("has a driftwood label on amber in both themes", () => {
    for (const theme of ["light", "dark"] as const) {
      expect(colors[theme]["on-act"]).toBe("#23211F");
      expect(colors[theme].act).toBe("#F2B33D");
      expect(contrast(colors[theme]["on-act"], colors[theme].act)).toBeGreaterThan(8.6 - 0.05);
    }
  });

  it("has a 2px driftwood edge that separates it from light surfaces", () => {
    const p = colors.light;
    expect(p["act-edge"]).toBe("#23211F");
    for (const bg of ["bg", "surface", "act-tint"] as const) {
      expect(contrast(p["act-edge"], p[bg])).toBeGreaterThanOrEqual(GRAPHIC);
    }
  });

  it("has no edge in dark mode, where amber itself stands out", () => {
    const p = colors.dark;
    expect(p["act-edge"]).toBe("transparent");
    for (const bg of ["bg", "surface", "act-tint"] as const) {
      expect(contrast(p.act, p[bg])).toBeGreaterThanOrEqual(GRAPHIC);
    }
  });
});
