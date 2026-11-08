import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
// @ts-expect-error plain JS module without types
import { renderTokensCss } from "../scripts/build-tokens.mjs";
import brand from "../src/tokens/brand.json";
import ui from "../src/tokens/ui.json";

const read = (p: string) => readFileSync(new URL(p, import.meta.url), "utf8");

describe("tokens", () => {
  it("tokens.css is up to date (run `pnpm tokens`)", () => {
    expect(read("../src/styles/tokens.css")).toBe(renderTokensCss(brand, ui));
  });

  it("light and dark define the same tokens", () => {
    expect(Object.keys({ ...brand.color.dark, ...ui.color.dark }).sort()).toEqual(
      Object.keys({ ...brand.color.light, ...ui.color.light }).sort(),
    );
  });

  it("app tokens never override brand-kit tokens", () => {
    for (const theme of ["light", "dark"] as const) {
      for (const key of Object.keys(ui.color[theme])) expect(brand.color[theme]).not.toHaveProperty(key);
    }
  });

  it("dark values apply for prefers-color-scheme and for data-theme=dark, light for data-theme=light", () => {
    const css = read("../src/styles/tokens.css");
    expect(css).toContain(':root:not([data-theme="light"])');
    expect(css).toContain(':root[data-theme="dark"]');
  });

  it("amber is used only by act-now styles in components.css (brand README rule 1)", () => {
    const css = read("../src/styles/components.css");
    const rules = css.split("}").filter((r) => /var\(--ww-act\)/.test(r));
    expect(rules.length).toBeGreaterThan(0);
    for (const r of rules) expect(r).toMatch(/\.ww-btn-act/);
  });

  it("act-now button uses the on-act label and act-edge border, and beats link colors", () => {
    const css = read("../src/styles/components.css");
    const rule = /([^}]*\.ww-btn-act[^{]*)\{([^}]*)\}/.exec(css);
    expect(rule).not.toBeNull();
    const [, selectors, body] = rule!;
    for (const sel of ["a.ww-btn-act", "a.ww-btn-act:hover", "a.ww-btn-act:visited"]) expect(selectors).toContain(sel);
    expect(body).toContain("background: var(--ww-act)");
    expect(body).toContain("color: var(--ww-on-act)");
    expect(body).toContain("border: 2px solid var(--ww-act-edge)");
  });
});
