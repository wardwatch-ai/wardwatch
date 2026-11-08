# @wardwatch/ui

The Wardwatch design system: brand tokens, light and dark themes, bundled fonts, and the shared React components the desktop app's screens are built from. It follows the brand kit (`/brand` in the project files) and the desktop design spec.

## Use it

```tsx
import "@wardwatch/ui/styles.css"; // tokens, fonts, base and component styles
import { Button, Card } from "@wardwatch/ui";

<Card tone="urgent">
  <h2>Update Chrome.</h2>
  <p>This is being exploited right now.</p>
  <Button variant="act">Update now</Button>
</Card>;
```

The package ships TypeScript source; the app's Vite build compiles it. With Tailwind 4, also import `@wardwatch/ui/tailwind.css` to get utilities such as `bg-surface` and `text-act-ink` that follow the theme.

## Themes

Light and dark follow the OS by default. `setTheme("light" | "dark" | "system")` pins one by setting `data-theme` on `<html>`. Every color is a `--ww-*` CSS variable.

## Buttons

| Variant | When |
|---|---|
| `act` | Only for a hole being exploited right now. Amber, driftwood label, 2px driftwood edge in light mode, no edge in dark (brand README rule 6). Works as a link too (`href`). |
| `quiet` + `severity` | Every other update button. 3-bar meter: 3 serious, 2 moderate, 1 minor or no security fix. Screen readers hear the severity. |
| `brand` | Ordinary primary actions (Invite, Share score, Done). |
| `quiet` | Secondary actions (Release hold, Hold, Details). |

## Tokens

`src/tokens/brand.json` is a copy of the brand kit's `tokens.json`. `src/tokens/ui.json` adds the app-only tokens from the final design canvas (link, ok-tint, sel, code, band colors, and `edge` for control outlines). After changing either, run `pnpm tokens` to regenerate `src/styles/tokens.css`.

Two values differ from the canvas so every pair passes WCAG: `sel` (light) is #ECE2CF instead of #EADFCB so muted text on a selected row reaches 4.5:1, and quiet buttons, chips and the account pill are outlined in `edge` rather than `line`, because `line` is 1.2:1 and controls need 3:1.

## Checks

- `pnpm test`: contrast for every text, control and graphic pair in both themes, the act-now rules, that amber is used only by act-now styles, generated tokens are current, component behavior and axe accessibility checks.
- `pnpm typecheck`
- `pnpm dev`: a local gallery of every component with a theme switch. It is never deployed.
