import brand from "./brand.json";
import ui from "./ui.json";

export type ThemeName = "light" | "dark";
type Palette = typeof brand.color.light & typeof ui.color.light;
export type ColorToken = keyof Palette;

/** Every color token for each theme, as hex (act-edge is "transparent" in dark). */
export const colors: Record<ThemeName, Palette> = {
  light: { ...brand.color.light, ...ui.color.light },
  dark: { ...brand.color.dark, ...ui.color.dark },
};

export const fonts = brand.font;
export const radius = brand.radius;

/** CSS reference for a token, e.g. cssVar("act-ink") === "var(--ww-act-ink)". */
export const cssVar = (token: ColorToken) => `var(--ww-${token})` as const;

export type ThemePreference = ThemeName | "system";

/**
 * Pins the theme on <html data-theme>, or follows the OS for "system".
 * tokens.css reads the attribute, so no React context is needed.
 */
export function setTheme(pref: ThemePreference, root: HTMLElement = document.documentElement) {
  if (pref === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", pref);
}
