import { readFileSync } from "node:fs";
import { browser, $, $$, expect } from "@wdio/globals";

const { version } = JSON.parse(
  readFileSync(new URL("../../apps/desktop/package.json", import.meta.url), "utf8"),
);
const os = { linux: "linux", win32: "windows", darwin: "macos" }[process.platform];
const tabs = ["Home", "Score", "Groups", "Apps", "Update plan"];

// Calls a Rust command the way the UI does, from inside the window.
// (Not `{ error }`: WebDriver clients read a result with an `error` key as a failure.)
const invoke = (cmd) =>
  browser.execute(
    (cmd) =>
      new Promise((resolve) => {
        window.__TAURI_INTERNALS__.invoke(cmd).then(
          (value) => resolve({ ok: true, value }),
          (reason) => resolve({ ok: false, reason: String(reason) }),
        );
      }),
    cmd,
  );

describe("Wardwatch desktop app", () => {
  // The app opens two webviews (main and the hidden tray popover), both titled
  // "Wardwatch". Drive the main one.
  before(async () => {
    for (const handle of await browser.getWindowHandles()) {
      await browser.switchToWindow(handle);
      if (!(await browser.getUrl()).includes("tray.html")) return;
    }
    throw new Error("no main window");
  });

  it("opens the main window", async () => {
    await expect(browser).toHaveTitle("Wardwatch");
    await expect($("main")).toBeDisplayed();
  });

  it("shows what the Rust core reports about itself", async () => {
    // The footer comes from the app_info command, so this proves the UI reached
    // the real core through IPC and the main window's capability grant.
    const footer = $("footer");
    await expect(footer).toHaveText(expect.stringContaining(`Wardwatch ${version} on ${os}`));
    await expect(footer).not.toHaveText(expect.stringContaining("Could not reach"));
  });

  it("opens every tab", async () => {
    for (const label of tabs) {
      const link = $("nav").$(`a=${label}`);
      await link.click();
      await expect(link).toHaveAttribute("aria-current", "page");
      await expect($$("nav a[aria-current=page]")).toBeElementsArrayOfSize(1);
      await expect($("main")).toBeDisplayed();
    }
  });

  it("refuses commands the main window isn't granted", async () => {
    const granted = await invoke("app_info");
    expect(granted).toEqual({ ok: true, value: expect.objectContaining({ version, os }) });
    // show_main_window is granted to the tray popover only (capabilities/tray.json).
    const refused = await invoke("show_main_window");
    expect(refused.ok).toBe(false);
    expect(refused.reason).toMatch(/not allowed/i);
  });
});
