# Design check

Holds the desktop app to the screens Matt approved on the
[final design canvas](https://claude.ai/artifact/Nv9Qdr7hGLdMpBoCFUuXvo).

Each screen in `cases.ts` is rendered from the production Vite build with the
browser sample scenario, in light and dark, at the canvas sizes (1280x900 for the
window, 400x740 for the menu bar popover), with a fixed clock and the bundled
brand fonts.

## The loop

Font rendering differs between Linux machines, so the images Matt approves and
the gate compares are always CI renders (`ubuntu-24.04`, Playwright's
Chromium). Local runs are for building and checking the review flow.

1. **Review.** When the gate fails, CI runs `pnpm visual:review` and uploads
   `out/` as the `visual-review` artifact: for each screen the mock, the
   app, a pixel diff and `<id>.json` (how much differs, the commit, and whether the
   screen changed since its last verdict). Claude turns that into a review page where
   Matt marks each screen "Looks right" or "Needs fixing", with a note.
2. **Record.** `pnpm visual:approve verdicts.json` checks that the review was rendered from
   the current commit, then copies the screens Matt saw
   into `approved/` and records each verdict in `approvals.json` (verdict, note,
   who, when, and the sha256 of the mock and app images). A "Needs fixing"
   screen is recorded too; its note is the work to do, and the fix comes back
   for review as a changed screen.
3. **Gate.** `pnpm visual` (CI, job "Screens match Matt's reviewed designs")
   fails when a screen has no verdict, when its mock changed since the verdict,
   when `approved/` holds an image other than the one Matt reviewed, or when
   any pixel of the app differs from the approved image (renders are
   byte-identical on the pinned platform). "Needs fixing" screens pass but are
   listed on every run. On failure CI uploads the review set as the `visual-review` artifact.

So any visual change to a screen needs Matt's eyes before it merges, and
nothing else does.

## Mocks

`mocks/src/` holds the approved artboards and `mocks/*.png` their renders. The
renders need the canvas runtime, which is not committed. When the canvas
changes, download its `artifact-type/dc-runtime.js` and the changed artboards,
then run `DC_RUNTIME=/path/to/dc-runtime.js pnpm visual:mocks`. A changed mock
fails the gate until Matt reviews the screen again.

## Adding a screen

Add a case to `cases.ts` with its artboard and route and render its mock. Push:
the gate fails on the new screen, and its CI render goes to Matt for review.
