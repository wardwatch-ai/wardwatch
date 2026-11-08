import { beforeEach, describe, expect, it, vi } from "vitest";
import { getAppInfo, getPlan, listFindings, scanInventory, setPlan, showMainWindow } from "./ipc";
import { samplePlan } from "./sample";

const core = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn() }));
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

  it("reports nothing checked inside Tauri until the matcher lands", async () => {
    core.isTauri.mockReturnValue(true);
    await expect(listFindings()).resolves.toBeNull();
  });
});
