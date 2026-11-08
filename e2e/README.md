# End-to-end tests for the desktop app

These drive the **real, built app**, with its real Rust core, through
[tauri-driver](https://v2.tauri.app/develop/tests/webdriver/) and
[WebdriverIO](https://webdriver.io). Unit tests fake the Rust side; these
check that the UI and the core actually talk, and that each window gets only
the commands its capability grants.

CI runs them on Linux (WebKitWebDriver). Windows would use Microsoft Edge
WebDriver, but WebView2 runtime 133 and later no longer let it open a session
(`DevToolsActivePort file doesn't exist`, see
[MicrosoftEdge/WebView2Feedback#5415](https://github.com/MicrosoftEdge/WebView2Feedback/issues/5415)
and [actions/runner-images#14738](https://github.com/actions/runner-images/issues/14738)),
and macOS has no WebDriver for its webview at all. Both are covered by unit
tests, unsigned builds and the catalog's real-scan jobs instead.

```sh
cargo install tauri-driver --version 2.1.0 --locked
sudo apt-get install webkit2gtk-driver xvfb    # Linux
pnpm install
pnpm --filter @wardwatch/desktop tauri build --debug --no-bundle
xvfb-run -a pnpm --filter @wardwatch/e2e e2e  # Linux; on Windows drop xvfb-run
```

On Windows (older WebView2 only), set `NATIVE_DRIVER` to an
`msedgedriver.exe` matching the installed WebView2 version. `WARDWATCH_APP` and `TAURI_DRIVER` override where the app and driver are.

Specs live in `specs/`. Screenshot comparison against the approved mocks is a
separate Playwright suite (in progress).
