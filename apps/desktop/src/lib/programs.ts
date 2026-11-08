// Turns the inventory, findings and holds into what the screens show for each program.
import type { Severity } from "@wardwatch/ui";
import type { FeedSeverity, Finding, Hold, Inventory, Kind, Source, Vuln } from "./model";

/**
 * What a program's detail corner shows (design spec, Apps):
 *  urgent   being exploited: amber Update now
 *  update   waiting for its plan slot: quiet Update now with the severity meter
 *  held     held by you: Release hold
 *  current  you have the newest: green Up to date
 *  unknown  not checked against the update feed yet: no claim either way
 */
export type ProgramState = "urgent" | "update" | "held" | "current" | "unknown";

export interface Program {
  key: string;
  /** For /fix/:source/:id and holds. The OS uses source "os". */
  source: Source | "os";
  id: string;
  name: string;
  vendor: string | null;
  kind: Kind;
  version: string;
  state: ProgramState;
  latest: string | null;
  /** Meter level for quiet update buttons: 3 serious, 2 moderate, 1 minor or no security fix. */
  severity: Severity;
  vulns: Vuln[];
  /** Set while the user holds it, including when it is urgent anyway (exploited). */
  heldSince: string | null;
}

export function meterLevel(severity: FeedSeverity | null): Severity {
  if (severity === "critical" || severity === "high") return 3;
  if (severity === "medium") return 2;
  return 1;
}

export function osKey(inventory: Inventory): string {
  return `os:${inventory.os.id}`;
}

function stateOf(f: Finding | undefined, held: boolean): ProgramState {
  if (!f) return "unknown";
  // A held program still alerts on a hole being exploited (Update plan rules).
  if (f.update_available && f.exploited) return "urgent";
  if (f.update_available) return held ? "held" : "update";
  return "current";
}

const rank: Record<ProgramState, number> = {
  urgent: 0,
  update: 1,
  held: 2,
  current: 3,
  unknown: 3,
};

/**
 * Builds this device's program list, the OS first among equals.
 * `findings` is null when nothing has been checked yet, so every program is "unknown".
 */
export function buildPrograms(
  inventory: Inventory,
  findings: Finding[] | null,
  holds: Hold[],
): Program[] {
  const byKey = new Map<string, Finding>();
  for (const f of findings ?? []) {
    byKey.set(`${f.source}:${f.software_id}`, f);
    for (const i of f.installs ?? []) byKey.set(`${i.source}:${i.id}`, f);
  }
  const heldSince = new Map(holds.map((h) => [`${h.source}:${h.id}`, h.since]));

  const entries = [
    {
      key: osKey(inventory),
      source: "os" as const,
      id: inventory.os.id,
      name: inventory.os.name,
      vendor: null,
      kind: "system" as const,
      version: inventory.os.version,
    },
    ...inventory.software.map((s) => ({
      key: `${s.source}:${s.id}`,
      source: s.source,
      id: s.id,
      name: s.name,
      vendor: s.publisher ?? null,
      kind: s.browser ? ("browsers" as const) : ("apps" as const),
      version: s.version ?? "unknown",
    })),
  ];

  return entries
    .map((e): Program => {
      const f = byKey.get(e.key);
      const since = heldSince.get(e.key) ?? null;
      const state = stateOf(f, since !== null);
      return {
        ...e,
        state,
        latest: f?.latest ?? null,
        severity: f && f.vulns.length > 0 ? meterLevel(f.severity) : 1,
        vulns: f?.vulns ?? [],
        heldSince: since,
      };
    })
    .sort((a, b) => rank[a.state] - rank[b.state] || a.name.localeCompare(b.name));
}

export interface DeviceStatus {
  checked: boolean;
  urgent: Program[];
  /** Waiting for the Sunday tune-up (or a later slot). */
  waiting: Program[];
  held: Program[];
  total: number;
  current: number;
  byKind: Record<Kind, { total: number; current: number; urgent: Program[]; programs: Program[] }>;
}

export function summarize(programs: Program[], checked: boolean): DeviceStatus {
  const byKind: DeviceStatus["byKind"] = {
    system: { total: 0, current: 0, urgent: [], programs: [] },
    browsers: { total: 0, current: 0, urgent: [], programs: [] },
    apps: { total: 0, current: 0, urgent: [], programs: [] },
  };
  for (const p of programs) {
    const k = byKind[p.kind];
    k.total += 1;
    k.programs.push(p);
    if (p.state === "current") k.current += 1;
    if (p.state === "urgent") k.urgent.push(p);
  }
  return {
    checked,
    urgent: programs.filter((p) => p.state === "urgent"),
    waiting: programs.filter((p) => p.state === "update"),
    held: programs.filter((p) => p.state === "held"),
    total: programs.length,
    current: programs.filter((p) => p.state === "current").length,
    byKind,
  };
}

/** The fix screen route for a program. */
export function fixPath(p: Pick<Program, "source" | "id">): string {
  return `/fix/${pathPart(p.source)}/${pathPart(p.id)}`;
}

/**
 * encodeURIComponent, plus the five characters it leaves alone (!'()*), so a route
 * holds only letters, digits, "-._~" and %XX: the set the core's allow-list accepts.
 * Windows ids are raw registry key names like "Firefox 145.0.1 (x64 en-US)".
 */
function pathPart(s: string): string {
  return encodeURIComponent(s).replace(
    /[!'()*]/g,
    (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`,
  );
}

/** "Zoom, VLC and macOS 16.1.1" */
export function listNames(programs: Program[]): string {
  const names = programs.map((p) =>
    p.kind === "system" && p.latest ? `${p.name} ${p.latest}` : p.name,
  );
  if (names.length <= 1) return names.join("");
  return `${names.slice(0, -1).join(", ")} and ${names.at(-1) ?? ""}`;
}

/** One line of "On this device" (Home) and the tray checklist. */
export function kindLine(
  status: DeviceStatus,
  kind: Kind,
): { mark: "urgent" | "current" | "none"; text: string } {
  const k = status.byKind[kind];
  if (!status.checked) return { mark: "none", text: `${String(k.total)} found` };
  if (k.urgent[0]) return { mark: "urgent", text: `${k.urgent[0].name} needs you` };
  if (k.current === k.total) return { mark: "current", text: "Current" };
  if (k.total === 1) {
    const only = k.programs[0];
    const text =
      only?.state === "held" ? "Held" : only?.state === "update" ? "Update waiting" : "Not checked";
    return { mark: "none", text };
  }
  return { mark: "none", text: `${String(k.current)} of ${String(k.total)} current` };
}
