import { beforeEach, describe, expect, it, vi } from "vitest";
import { getAppInfo, showMainWindow } from "./ipc";

const core = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => core);

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
    await showMainWindow();
    expect(core.invoke).not.toHaveBeenCalled();
  });

  it("asks the core to show the main window", async () => {
    core.isTauri.mockReturnValue(true);
    await showMainWindow();
    expect(core.invoke).toHaveBeenCalledWith("show_main_window");
  });
});
