// Drives the real desktop app (the debug binary, with its real Rust core)
// through tauri-driver, Tauri's official WebDriver bridge.
// Linux uses WebKitWebDriver (this is what CI runs). Windows would use
// Microsoft Edge WebDriver, but WebView2 133+ no longer opens a session for it
// (MicrosoftEdge/WebView2Feedback#5415), so Windows isn't in CI. macOS has no
// WebDriver for its webview at all.
// TODO: move to @wdio/tauri-service (spawns the driver, waits for it, fetches
// a matching Edge WebDriver) once a stable release supports WebdriverIO 10.
//
// Build first:  pnpm --filter @wardwatch/desktop tauri build --debug --no-bundle
// Then:         pnpm --filter @wardwatch/e2e e2e   (on Linux, under xvfb-run)
import { spawn } from "node:child_process";
import { homedir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const exe = process.platform === "win32" ? ".exe" : "";
const application = process.env.WARDWATCH_APP ?? join(root, "target", "debug", `wardwatch${exe}`);
const tauriDriver =
  process.env.TAURI_DRIVER ?? join(homedir(), ".cargo", "bin", `tauri-driver${exe}`);
// On Windows, an Edge WebDriver that matches the WebView2 runtime.
const nativeDriver = process.env.NATIVE_DRIVER;

let driver;

export const config = {
  runner: "local",
  specs: ["./specs/**/*.js"],
  maxInstances: 1,
  hostname: "127.0.0.1",
  port: 4444,
  capabilities: [{ maxInstances: 1, "tauri:options": { application } }],
  logLevel: "warn",
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: { ui: "bdd", timeout: 60_000 },
  waitforTimeout: 15_000,

  beforeSession() {
    const args = nativeDriver ? ["--native-driver", nativeDriver] : [];
    driver = spawn(tauriDriver, args, { stdio: [null, process.stdout, process.stderr] });
  },
  afterSession() {
    driver?.kill();
  },
};
