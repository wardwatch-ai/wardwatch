import { describe, expect, it } from "vitest";
import { hourLabel, matchingPreset, presets, withPreset, withRule } from "./plan";
import { samplePlan } from "./sample";

describe("update plan", () => {
  it("recognizes the preset the grid matches", () => {
    expect(matchingPreset(samplePlan.rules)).toBe("balanced");
    expect(matchingPreset(withPreset(samplePlan, "eager").rules)).toBe("eager");
  });

  it("matches no preset once a box differs, and again when it's put back", () => {
    const edited = withRule(samplePlan, "minor", "apps", "auto");
    expect(matchingPreset(edited.rules)).toBeNull();
    expect(matchingPreset(withRule(edited, "minor", "apps", "monthly").rules)).toBe("balanced");
  });

  it("doesn't share preset objects with plans", () => {
    const plan = withPreset(samplePlan, "relaxed");
    plan.rules.serious.apps = "auto";
    expect(presets.relaxed.rules.serious.apps).toBe("sunday_tuneup");
  });

  it("labels hours", () => {
    expect(["00:00", "08:00", "12:00", "22:00"].map(hourLabel)).toEqual([
      "Midnight",
      "8 am",
      "Noon",
      "10 pm",
    ]);
  });
});
