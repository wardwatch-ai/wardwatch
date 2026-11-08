# End-to-end tests for the desktop app

These drive the **real, built app**, with its real Rust core, through
[tauri-driver](https://v2.tauri.app/develop/tests/webdriver/) and
[WebdriverIO](https://webdriver.io). Unit tests fake the Rust side; these
check that the UI and the core actually talk, and that each window gets only
the commands its capability grants.

Linux uses WebKitWebDriver, Windows uses Microsoft Edge WebDriver. macOS has
no WebDriver for its webview, so macOS is covered by unit tests and the
catalog's real-scan job instead.

```sh
cargo install tauri-driver --version 2.1.0 --locked
sudo apt-get install webkit2gtk-driver xvfb    # Linux
pnpm install
pnpm --filter @wardwatch/desktop tauri build --debug --no-bundle
xvfb-run -a pnpm --filter @wardwatch/e2e e2e  # Linux; on Windows drop xvfb-run
```

On Windows set `NATIVE_DRIVER` to an `msedgedriver.exe` that matches the
installed WebView2 version (CI downloads one, see `.github/workflows/e2e.yml`).
`WARDWATCH_APP` and `TAURI_DRIVER` override where the app and driver are.

Specs live in `specs/`. Screenshot comparison against the approved mocks is a
separate suite in `apps/desktop/e2e/visual/` (Playwright).
