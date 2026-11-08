import { beforeEach, describe, expect, it, vi } from "vitest";
import { getAppInfo, getFix, listFindings, runFix, showMainWindow } from "./ipc";

const core = vi.hoisted(() => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
  Channel: class {
    onmessage: (m: unknown) => void = () => undefined;
  },
}));
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
        return Promise.resolve({ outcome: { status: "installed" }, closed: true, finding: null });
      },
    );
    await expect(runFix("dpkg", "openssl", (p) => seen.push(p))).resolves.toMatchObject({
      closed: true,
    });
    expect(core.invoke).toHaveBeenLastCalledWith(
      "run_fix",
      expect.objectContaining({ source: "dpkg", id: "openssl" }),
    );
    expect(seen).toEqual([{ event: "step", step: 0, state: "working" }]);
  });
});
