// Typed wrappers around the Rust commands in src-tauri/src/commands.rs.
// Every call from the UI to the core goes through this file, so the
// command names and payload shapes live in one place on each side.
import { invoke, isTauri } from "@tauri-apps/api/core";

export interface AppInfo {
  name: string;
  version: string;
  os: string;
}

const browserFallback: AppInfo = { name: "Wardwatch", version: "dev", os: "browser" };

export async function getAppInfo(): Promise<AppInfo> {
  // Lets `pnpm dev` run the UI in a plain browser while designing screens.
  if (!isTauri()) return browserFallback;
  return invoke<AppInfo>("app_info");
}

export async function showMainWindow(): Promise<void> {
  if (!isTauri()) return;
  await invoke("show_main_window");
}
