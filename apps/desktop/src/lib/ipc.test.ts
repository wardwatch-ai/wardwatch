import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getAppInfo,
  getFix,
  getNudges,
  getPlan,
  listFindings,
  runFix,
  scanInventory,
  setPlan,
  showMainWindow,
} from "./ipc";
import { samplePlan } from "./sample";

const core = vi.hoisted(() => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
  Channel: class {
    onmessage: (m: unknown) => void = () => undefined;
  },
}));
vi.mock("@tauri-apps/api/core", () => core);
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

describe("ipc", () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it("calls the app_info command inside Tauri", async () => {
    core.isTauri.mockReturnValue(true);
    core.invoke.mockResolvedValue({ name: "Wardwatch", version: "0.1.0", os: "linux" });
    await expect(getAppInfo()).resolves.toEqual({
      name: "Wardwatch",
      version: "0.1.0",
      os: "linux",
    });
    expect(core.invoke).toHaveBeenCalledWith("app_info");
  });

  it("does not touch IPC in a plain browser", async () => {
    core.isTauri.mockReturnValue(false);
    await expect(getAppInfo()).resolves.toMatchObject({ os: "browser" });
    await showMainWindow("/score");
    await expect(scanInventory()).resolves.toMatchObject({ os: { name: "macOS" } });
    await expect(listFindings()).resolves.not.toBeNull();
    await expect(getNudges()).resolves.toMatchObject({ top_card: { kind: "under_attack" } });
    await setPlan({ ...samplePlan, holds: [] });
    await expect(getPlan()).resolves.toMatchObject({ holds: [] });
    expect(core.invoke).not.toHaveBeenCalled();
  });

  it("asks the core to show the main window, optionally on a screen", async () => {
    core.isTauri.mockReturnValue(true);
    await showMainWindow();
    expect(core.invoke).toHaveBeenLastCalledWith("show_main_window", {});
    await showMainWindow("/fix/mac_app/com.google.Chrome");
    expect(core.invoke).toHaveBeenLastCalledWith("show_main_window", {
      route: "/fix/mac_app/com.google.Chrome",
    });
  });

  it("scans and stores the plan through the core", async () => {
    core.isTauri.mockReturnValue(true);
    await scanInventory(true);
    expect(core.invoke).toHaveBeenLastCalledWith("scan_inventory", { refresh: true });
    await setPlan(samplePlan);
    expect(core.invoke).toHaveBeenLastCalledWith("set_plan", { plan: samplePlan });
    await getPlan();
    expect(core.invoke).toHaveBeenLastCalledWith("get_plan");
  });

  it("reports nothing checked when the core can't match (no catalog yet)", async () => {
    core.isTauri.mockReturnValue(true);
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    core.invoke.mockRejectedValue("Wardwatch hasn't received a signed vulnerability catalog yet.");
    await expect(listFindings()).resolves.toBeNull();
    await expect(getNudges()).resolves.toBeNull();
    expect(core.invoke).toHaveBeenLastCalledWith("get_nudges");
  });

  it("names a program to the fix commands and streams progress over a channel", async () => {
    core.isTauri.mockReturnValue(true);
    core.invoke.mockResolvedValue([]);
    await listFindings();
    expect(core.invoke).toHaveBeenCalledWith("list_findings");
    await getFix("dpkg", "openssl");
    expect(core.invoke).toHaveBeenCalledWith("get_fix", { source: "dpkg", id: "openssl" });

    const seen: unknown[] = [];
    core.invoke.mockImplementation(
      (_cmd: string, args: { onProgress: { onmessage: (m: unknown) => void } }) => {
        args.onProgress.onmessage({ event: "step", step: 0, state: "working" });
        return Promise.resolve({
          outcome: { status: "installed" },
          check: "closed",
          finding: null,
        });
      },
    );
    await expect(
      runFix("dpkg", "openssl", { kind: "app_updater", open_bundle: null }, (p) => seen.push(p)),
    ).resolves.toMatchObject({
      check: "closed",
    });
    expect(core.invoke).toHaveBeenLastCalledWith(
      "run_fix",
      expect.objectContaining({ source: "dpkg", id: "openssl" }),
    );
    expect(seen).toEqual([{ event: "step", step: 0, state: "working" }]);
  });
});
