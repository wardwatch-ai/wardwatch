// Sets or checks the app version for a release build. The version lives in
// package.json (Tauri reads it) and src-tauri/Cargo.toml; this keeps them equal.
//
//   node scripts/release-version.ts nightly [YYYY-MM-DD]   set the nightly version, print it
//   node scripts/release-version.ts check vX.Y.Z           fail unless both files say X.Y.Z
//
// A nightly is the next minor release plus days since 2026-01-01, e.g. 0.2.0-312
// while 0.1.x is current. It sorts above every 0.1.x and below 0.2.0, and each
// night's number is higher than the last. The number stays numeric because the
// Windows installer (MSI) only accepts a numeric pre-release up to 65535.
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const epoch = Date.UTC(2026, 0, 1);
const release = /^(\d+)\.(\d+)\.(\d+)$/;

export function nightlyVersion(current: string, day: string): string {
  const m = release.exec(current);
  if (!m) throw new Error(`expected a release version like 0.1.0, not ${current}`);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(day)) throw new Error(`expected YYYY-MM-DD, not ${day}`);
  const days = (Date.parse(`${day}T00:00:00Z`) - epoch) / 86_400_000;
  if (!Number.isInteger(days) || days < 0 || days > 65_535) {
    throw new Error(`${day} is outside the nightly numbering`);
  }
  return `${m[1]}.${String(Number(m[2]) + 1)}.0-${String(days)}`;
}

/** The [package] version in a Cargo.toml. */
export function cargoVersion(toml: string): string | undefined {
  return /^\[package\][^[]*?^version = "([^"]+)"/m.exec(toml)?.[1];
}

export function withCargoVersion(toml: string, version: string): string {
  if (cargoVersion(toml) === undefined) throw new Error("no [package] version in Cargo.toml");
  return toml.replace(/^(\[package\][^[]*?^version = ")[^"]+(")/m, `$1${version}$2`);
}

function main(args: string[]) {
  const root = join(fileURLToPath(import.meta.url), "../..");
  const pkgPath = join(root, "package.json");
  const cargoPath = join(root, "src-tauri/Cargo.toml");
  const pkg = JSON.parse(readFileSync(pkgPath, "utf8")) as { version: string };
  const toml = readFileSync(cargoPath, "utf8");
  const [command] = args;
  const arg = args.length > 1 ? args[1] : undefined;

  if (command === "nightly") {
    const day = arg ?? new Date().toISOString().slice(0, 10);
    const version = nightlyVersion(pkg.version, day);
    writeFileSync(pkgPath, JSON.stringify({ ...pkg, version }, null, 2) + "\n");
    writeFileSync(cargoPath, withCargoVersion(toml, version));
    console.log(version);
  } else if (command === "check" && arg) {
    const want = arg.replace(/^v/, "");
    const have = [pkg.version, cargoVersion(toml)];
    if (!release.test(want)) throw new Error(`mainline tags look like v1.2.3, not ${arg}`);
    if (have.some((v) => v !== want)) {
      throw new Error(`tag ${arg} but package.json and Cargo.toml say ${have.join(" and ")}`);
    }
    console.log(want);
  } else {
    throw new Error("usage: release-version.ts nightly [YYYY-MM-DD] | check vX.Y.Z");
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2));
}
