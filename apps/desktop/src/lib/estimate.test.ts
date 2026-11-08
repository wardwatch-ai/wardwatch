import { describe, expect, it } from "vitest";
import { daysToSunday, pendingFix, pointsForFixingNow } from "./estimate";
import type { Program } from "./programs";

const sums = { total: 40, in_time: 36, speed: 31.2 }; // 90 up to date, 78 fix speed: 84
const chrome = { exploited: true, severity: "high" as const, popular: true };
const zoom = { exploited: false, severity: "medium" as const, popular: false };

describe("pointsForFixingNow", () => {
  it("counts an exploited fix today against tomorrow", () => {
    // Today: 4 more in time and full speed. Tomorrow: still in time, speed e^-0.5.
    expect(pointsForFixingNow(sums, 1, [chrome], 1)).toBe(2);
  });

  it("counts a routine fix now against waiting for Sunday", () => {
    expect(pointsForFixingNow(sums, 1, [zoom, zoom], 5)).toBe(1);
    expect(pointsForFixingNow(sums, 1, [zoom], 0)).toBe(0);
  });

  it("works for a new account, whose sums are zero", () => {
    // Today: 100. Tomorrow: in time, speed e^-0.5 = 61, score 81.
    expect(pointsForFixingNow({ total: 0, in_time: 0, speed: 0 }, 1, [chrome], 1)).toBe(19);
  });

  it("gives no estimate without sums, for another algorithm, or for nothing", () => {
    expect(pointsForFixingNow(undefined, 1, [chrome], 1)).toBeNull();
    expect(pointsForFixingNow(sums, 2, [chrome], 1)).toBeNull();
    expect(pointsForFixingNow(sums, 1, [], 1)).toBeNull();
  });
});

describe("daysToSunday", () => {
  it("counts down to Sunday and is 0 on Sunday", () => {
    expect(daysToSunday(new Date(2026, 10, 8))).toBe(0); // Sunday
    expect(daysToSunday(new Date(2026, 10, 9))).toBe(6); // Monday
    expect(daysToSunday(new Date(2026, 10, 14))).toBe(1); // Saturday
  });
});

describe("pendingFix", () => {
  const program = (vulns: Program["vulns"], state: Program["state"] = "update") =>
    ({ kind: "apps", state, vulns }) as Program;
  const vuln = (severity: Program["vulns"][number]["severity"]) => ({
    id: "CVE-1",
    summary: "",
    severity,
    exploited: false,
  });

  it("skips updates that fix no known hole", () => {
    expect(pendingFix(program([]))).toBeNull();
  });

  it("takes the worst hole, and treats unrated holes as moderate", () => {
    expect(pendingFix(program([vuln("low"), vuln("high")]))?.severity).toBe("high");
    const unrated = pendingFix(program([vuln(null)]));
    expect(unrated?.severity).toBe("unknown");
    // Scored like a moderate fix: same as a medium one.
    const medium = pendingFix(program([vuln("medium")]));
    expect(unrated && medium && pointsForFixingNow(sums, 1, [unrated], 10)).toBe(
      medium && pointsForFixingNow(sums, 1, [medium], 10),
    );
  });
});
