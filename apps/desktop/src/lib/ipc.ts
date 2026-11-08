// Typed wrappers around the Rust commands in src-tauri/src/commands.rs.
// Every call from the UI to the core goes through this file, so the
// command names and payload shapes live in one place on each side.
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Finding, Inventory, UpdatePlan } from "./model";
import { sampleFindings, sampleInventory, samplePlan } from "./sample";

export interface AppInfo {
  name: string;
  version: string;
  os: string;
}

const browserFallback: AppInfo = { name: "Wardwatch", version: "dev", os: "browser" };

// Outside Tauri (`pnpm dev` in a browser, and tests) the UI runs on the design
// spec's sample scenario, so screens can be designed and checked without the core.

export async function getAppInfo(): Promise<AppInfo> {
  if (!isTauri()) return browserFallback;
  return invoke<AppInfo>("app_info");
}

/** Opens the main window, optionally on a screen such as "/score" or a fix route. */
export async function showMainWindow(route?: string): Promise<void> {
  if (!isTauri()) return;
  await invoke("show_main_window", route === undefined ? {} : { route });
}

/** Calls `handler` when the tray asks the main window to switch screens. */
export function onNavigate(handler: (route: string) => void): () => void {
  if (!isTauri()) return () => undefined;
  const stop = listen<string>("navigate", (e) => {
    handler(e.payload);
  });
  return () => {
    void stop.then((unlisten) => {
      unlisten();
    });
  };
}

/** This device's OS and installed software. The core caches the last scan for 10 minutes. */
export async function scanInventory(refresh = false): Promise<Inventory> {
  if (!isTauri()) return structuredClone(sampleInventory);
  return invoke<Inventory>("scan_inventory", { refresh });
}

/**
 * This device's programs matched against the update feed, or null when nothing
 * has been checked yet. The matcher (crates/wardwatch-feed and its `list_findings`
 * command) is being built separately; until it lands the real app reports null,
 * so no screen claims a program is up to date when nobody checked.
 */
export function listFindings(): Promise<Finding[] | null> {
  return Promise.resolve(isTauri() ? null : structuredClone(sampleFindings));
}

let browserPlan = structuredClone(samplePlan);

export async function getPlan(): Promise<UpdatePlan> {
  if (!isTauri()) return structuredClone(browserPlan);
  return invoke<UpdatePlan>("get_plan");
}

export async function setPlan(plan: UpdatePlan): Promise<void> {
  if (!isTauri()) {
    browserPlan = structuredClone(plan);
    return;
  }
  await invoke("set_plan", { plan });
}
