import { describe, expect, it } from "vitest";
import { cargoVersion, nightlyVersion, withCargoVersion } from "./release-version";

describe("nightlyVersion", () => {
  it("is the next minor release plus days since 2026-01-01", () => {
    expect(nightlyVersion("0.1.0", "2026-01-01")).toBe("0.2.0-0");
    expect(nightlyVersion("0.1.4", "2026-11-09")).toBe("0.2.0-312");
    expect(nightlyVersion("1.9.0", "2026-11-10")).toBe("1.10.0-313");
  });

  it("refuses versions and days it can't number", () => {
    expect(() => nightlyVersion("0.2.0-312", "2026-11-09")).toThrow();
    expect(() => nightlyVersion("0.1.0", "2025-12-31")).toThrow();
    expect(() => nightlyVersion("0.1.0", "Nov 9")).toThrow();
  });
});

describe("Cargo.toml version", () => {
  const toml =
    '[package]\nname = "x"\nversion = "0.1.0"\n\n[dependencies]\nserde = { version = "1" }\n';

  it("reads and replaces only the package version", () => {
    expect(cargoVersion(toml)).toBe("0.1.0");
    const updated = withCargoVersion(toml, "0.2.0-312");
    expect(cargoVersion(updated)).toBe("0.2.0-312");
    expect(updated).toContain('serde = { version = "1" }');
  });
});
