// Typed wrappers around the Rust commands in src-tauri/src/commands.rs.
// Every call from the UI to the core goes through this file, so the
// command names and payload shapes live in one place on each side.
import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Finding as ScreenFinding, Inventory, NudgePlan, UpdatePlan } from "./model";
import { sampleFindings, sampleInventory, sampleNudges, samplePlan } from "./sample";

export interface AppInfo {
  name: string;
  version: string;
  os: string;
  /** The channel this build was made for. Local builds are "dev". */
  channel: BuildChannel;
  /** Short git commit id, or "" when unknown. */
  commit: string;
  /** Build date, YYYY-MM-DD, or "" for a local build. */
  built: string;
}

export type ReleaseChannel = "mainline" | "nightly";
export type BuildChannel = ReleaseChannel | "dev";

const browserFallback: AppInfo = {
  name: "Wardwatch",
  version: "dev",
  os: "browser",
  channel: "dev",
  commit: "",
  built: "",
};

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
 * could be checked (no signed vulnerability catalog yet), so no screen claims a
 * program is up to date when nobody checked.
 */
export async function listFindings(): Promise<ScreenFinding[] | null> {
  if (!isTauri()) return structuredClone(sampleFindings);
  try {
    return await invoke<Finding[]>("list_findings");
  } catch (e) {
    console.warn("findings unavailable:", e);
    return null;
  }
}

/**
 * How hard and when to nudge for each update (crates/wardwatch-nudge), or null
 * when nothing could be checked, like listFindings.
 */
export async function getNudges(): Promise<NudgePlan | null> {
  if (!isTauri()) return structuredClone(sampleNudges);
  try {
    return await invoke<NudgePlan>("get_nudges");
  } catch (e) {
    console.warn("nudge plan unavailable:", e);
    return null;
  }
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

// ---- Findings and the 1-click fix (src-tauri/src/fix.rs) -------------------
// Shapes mirror crates/wardwatch-feed (Finding) and crates/wardwatch-update (Plan).

export type Severity = "critical" | "high" | "medium" | "low" | "unknown";

export interface Install {
  source: string;
  id: string;
  name: string;
  version: string;
  running: boolean;
}

export interface VulnHit {
  id: string;
  aliases: string[];
  published: string;
  severity: Severity;
  cvss: number | null;
  exploited: boolean;
  kev: { added: string; due: string; ransomware: boolean } | null;
  summary: string;
  /** null: no fix yet. */
  fixed: string | null;
}

export interface Finding {
  key: string;
  app_id: string;
  app_name: string;
  kind: "app" | "os" | "package";
  source: string;
  software_id: string;
  name: string;
  browser: string | null;
  version_scheme: "dotted" | "deb" | "rpm";
  installed: string;
  running: boolean;
  installs: Install[];
  fixed_version: string | null;
  latest: string | null;
  update_available: boolean;
  exploited: boolean;
  /** null: no known hole affects the installed version. */
  severity: Severity | null;
  vulns: VulnHit[];
}

export type UpdateTool = "homebrew" | "winget" | "apt" | "dnf" | "snap" | "flatpak";

export type Method =
  | { kind: "command"; tool: UpdateTool; argv: string[] }
  | { kind: "system_settings"; target: string }
  | { kind: "app_updater"; open_bundle: string | null };

export interface Plan {
  method: Method;
  by: string;
  one_click: boolean;
  admin: "no" | "asks" | "may_ask";
  steps: { label: string; detail: string }[];
  verifies: string;
  manual: string | null;
  restart_note: string | null;
}

export interface FixView {
  finding: Finding;
  plan: Plan | null;
  no_plan: { kind: "up_to_date" } | { kind: "unsupported"; why: string } | null;
}

export type FixProgress = { event: "step"; step: number; state: "working" | "done" };

export type Outcome =
  | { status: "installed" }
  | { status: "opened" }
  | { status: "cancelled" }
  | { status: "nothing_newer"; tail: string[] }
  | { status: "needs_more_changes"; tail: string[] }
  | { status: "failed"; message: string; tail: string[] };

export interface FixResult {
  outcome: Outcome;
  /** After an install, what the rescan found. */
  check: "closed" | "only_unfixed_left" | "still_open" | "not_found" | null;
  finding: Finding | null;
}

export async function getFix(source: string, id: string): Promise<FixView> {
  if (!isTauri()) return sample;
  return invoke<FixView>("get_fix", { source, id });
}

/**
 * Runs the update. The core replans from its own scan and never runs anything
 * the UI sends; `shown` is the method the screen showed, and the core refuses
 * to run if its fresh plan differs.
 */
export async function runFix(
  source: string,
  id: string,
  shown: Method,
  onProgress: (p: FixProgress) => void,
): Promise<FixResult> {
  if (!isTauri()) return simulateFix(onProgress);
  const channel = new Channel<FixProgress>();
  channel.onmessage = onProgress;
  return invoke<FixResult>("run_fix", { source, id, shown, onProgress: channel });
}

// The design spec's sample scenario, so `pnpm dev` shows the fix screen in a browser.
const sample: FixView = {
  finding: {
    key: "google-chrome",
    app_id: "google-chrome",
    app_name: "Google Chrome",
    kind: "app",
    source: "mac_app",
    software_id: "com.google.Chrome",
    name: "Google Chrome",
    browser: "chrome",
    version_scheme: "dotted",
    installed: "142.0.7444.59",
    running: true,
    installs: [
      {
        source: "mac_app",
        id: "com.google.Chrome",
        name: "Google Chrome",
        version: "142.0.7444.59",
        running: true,
      },
      {
        source: "homebrew_cask",
        id: "google-chrome",
        name: "google-chrome",
        version: "142.0.7444.59",
        running: false,
      },
    ],
    fixed_version: "142.0.7444.162",
    latest: "142.0.7444.162",
    update_available: true,
    exploited: true,
    severity: "high",
    vulns: [
      {
        id: "CVE-2026-13224",
        aliases: [],
        published: "2026-11-05",
        severity: "high",
        cvss: 8.8,
        exploited: true,
        kev: { added: "2026-11-06", due: "2026-11-27", ransomware: false },
        summary:
          "Type confusion in V8 in Google Chrome allowed a remote attacker to run code through a crafted HTML page.",
        fixed: "142.0.7444.162",
      },
    ],
  },
  plan: {
    method: {
      kind: "command",
      tool: "homebrew",
      argv: ["/opt/homebrew/bin/brew", "upgrade", "--cask", "--greedy", "google-chrome"],
    },
    by: "Homebrew",
    one_click: true,
    admin: "no",
    steps: [
      {
        label: "Download Google Chrome 142.0.7444.162",
        detail: "From the vendor's own download, through Homebrew",
      },
      {
        label: "Make sure it's the real thing",
        detail: "Homebrew checks the file's fingerprint, and macOS checks the app's signature",
      },
      { label: "Install it, then restart Google Chrome", detail: "Nothing to type" },
      { label: "Check again that the hole is closed", detail: "Wardwatch rescans this device" },
    ],
    verifies:
      "Homebrew downloads from the vendor and checks the file's SHA-256 fingerprint; macOS then checks the vendor's signature when the app opens.",
    manual: null,
    restart_note:
      "Google Chrome is open. It keeps running until you restart it, then the new version starts.",
  },
  no_plan: null,
};

async function simulateFix(onProgress: (p: FixProgress) => void): Promise<FixResult> {
  const wait = () => new Promise((r) => setTimeout(r, 1200));
  for (const step of [0, 1, 2]) onProgress({ event: "step", step, state: "working" });
  await wait();
  for (const step of [0, 1, 2]) onProgress({ event: "step", step, state: "done" });
  onProgress({ event: "step", step: 3, state: "working" });
  await wait();
  onProgress({ event: "step", step: 3, state: "done" });
  return { outcome: { status: "installed" }, check: "closed", finding: null };
}

// ---- Settings (src-tauri/src/settings.rs) -----------------------------------

export interface Settings {
  v: 1;
  channel: ReleaseChannel;
}

let browserSettings: Settings = { v: 1, channel: "mainline" };

export async function getSettings(): Promise<Settings> {
  if (!isTauri()) return { ...browserSettings };
  return invoke<Settings>("get_settings");
}

export async function setSettings(settings: Settings): Promise<void> {
  if (!isTauri()) {
    browserSettings = { ...settings };
    return;
  }
  await invoke("set_settings", { settings });
}
