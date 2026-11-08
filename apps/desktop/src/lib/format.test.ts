import { describe, expect, it } from "vitest";
import { buildDate, ordinal, releaseLabel } from "./format";

describe("ordinal", () => {
  it("handles teens", () => {
    expect([1, 2, 3, 4, 11, 12, 13, 21, 22, 101, 111].map(ordinal)).toEqual([
      "1st",
      "2nd",
      "3rd",
      "4th",
      "11th",
      "12th",
      "13th",
      "21st",
      "22nd",
      "101st",
      "111th",
    ]);
  });
});

describe("releaseLabel", () => {
  it("spells out nightlies and leaves other versions alone", () => {
    expect(releaseLabel({ version: "0.2.0-312", channel: "nightly" })).toBe("0.2.0 nightly 312");
    expect(releaseLabel({ version: "0.2.0", channel: "mainline" })).toBe("0.2.0");
    expect(releaseLabel({ version: "0.1.0", channel: "dev" })).toBe("0.1.0");
  });
});

describe("buildDate", () => {
  it("formats a UTC day and passes through anything else", () => {
    expect(buildDate("2026-11-09")).toBe("Nov 9, 2026");
    expect(buildDate("soon")).toBe("soon");
  });
});
