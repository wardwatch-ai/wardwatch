import { describe, expect, it } from "vitest";
import { buildPrograms, fixPath, listNames, meterLevel, summarize } from "./programs";
import { sampleFindings, sampleInventory, samplePlan } from "./sample";

const programs = () => buildPrograms(sampleInventory, sampleFindings, samplePlan.holds);
const byName = (name: string) => programs().find((p) => p.name === name);

describe("buildPrograms", () => {
  it("puts the exploited program first and classifies each state", () => {
    const list = programs();
    expect(list[0]?.name).toBe("Google Chrome");
    expect(byName("Google Chrome")).toMatchObject({
      state: "urgent",
      kind: "browsers",
      latest: "142.0.7444.162",
    });
    expect(byName("Zoom")).toMatchObject({ state: "update", severity: 2, kind: "apps" });
    expect(byName("macOS")).toMatchObject({
      state: "update",
      kind: "system",
      source: "os",
      id: "macos",
    });
    expect(byName("Final Cut Pro")).toMatchObject({ state: "held", heldSince: "2026-11-02" });
    expect(byName("Slack")).toMatchObject({ state: "update", severity: 1, vulns: [] });
    expect(byName("Firefox")).toMatchObject({ state: "current" });
  });

  it("claims nothing when nothing was checked", () => {
    const list = buildPrograms(sampleInventory, null, []);
    expect(list.every((p) => p.state === "unknown")).toBe(true);
    expect(summarize(list, false)).toMatchObject({
      checked: false,
      urgent: [],
      current: 0,
      total: 10,
    });
  });

  it("still alerts on a held program that is being exploited", () => {
    const holds = [{ source: "mac_app" as const, id: "com.google.Chrome", since: "2026-11-01" }];
    expect(buildPrograms(sampleInventory, sampleFindings, holds)[0]).toMatchObject({
      state: "urgent",
      heldSince: null,
    });
  });

  it("applies a finding to every install it covers", () => {
    const inventory = {
      ...sampleInventory,
      software: [
        {
          source: "dpkg" as const,
          id: "openssl",
          name: "openssl",
          version: "3.0.13-0ubuntu3.4",
          running: false,
        },
        {
          source: "dpkg" as const,
          id: "libssl3t64",
          name: "libssl3t64",
          version: "3.0.13-0ubuntu3.4",
          running: false,
        },
      ],
    };
    const finding = {
      key: "openssl",
      source: "dpkg",
      software_id: "openssl",
      installs: [
        { source: "dpkg", id: "openssl" },
        { source: "dpkg", id: "libssl3t64" },
      ],
      installed: "3.0.13-0ubuntu3.4",
      fixed_version: "3.0.13-0ubuntu3.5",
      latest: "3.0.13-0ubuntu3.5",
      update_available: true,
      exploited: false,
      severity: "critical" as const,
      vulns: [{ id: "CVE-1", summary: "x", severity: "critical" as const, exploited: false }],
    };
    const list = buildPrograms(inventory, [finding], []);
    expect(list.filter((p) => p.source === "dpkg").map((p) => [p.id, p.state, p.severity])).toEqual(
      [
        ["libssl3t64", "update", 3],
        ["openssl", "update", 3],
      ],
    );
  });
});

describe("helpers", () => {
  it("maps feed severities to meter bars", () => {
    expect([
      meterLevel("critical"),
      meterLevel("high"),
      meterLevel("medium"),
      meterLevel("low"),
      meterLevel(null),
    ]).toEqual([3, 3, 2, 1, 1]);
  });

  it("summarizes this device", () => {
    const s = summarize(programs(), true);
    expect(s.urgent.map((p) => p.name)).toEqual(["Google Chrome"]);
    expect(s.waiting.map((p) => p.name)).toEqual(["macOS", "Slack", "VLC", "Zoom"]);
    expect(s.byKind.browsers).toMatchObject({ total: 3, current: 2 });
  });

  it("encodes fix routes and lists names", () => {
    expect(fixPath({ source: "dpkg", id: "libssl3:amd64" })).toBe("/fix/dpkg/libssl3%3Aamd64");
    const waiting = summarize(programs(), true).waiting.filter((p) => p.name !== "Slack");
    expect(listNames(waiting)).toBe("macOS 16.1.1, VLC and Zoom");
  });
});
