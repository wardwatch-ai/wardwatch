# wardwatch-feed

Reads Wardwatch's update and vulnerability catalog (`catalog.json`, built by
`catalog/` in the private backend repo) and matches it against this device's
inventory from [`wardwatch-catalog`](../wardwatch-catalog). It's steps 2 and 3
of the core loop: compare what's installed against available updates, and map
those updates to the holes they close. Everything happens on the device.

```rust
let catalog = wardwatch_feed::Catalog::from_json(&bytes, SystemTime::now())?; // schema 1, not expired
let findings = wardwatch_feed::match_inventory(&wardwatch_catalog::scan(), &catalog);
```

## Findings

One `Finding` per catalog app that's installed, most urgent first (exploited,
then worst severity, then name). Serialized with serde:

```jsonc
{
  "key": "mac_app:com.google.Chrome",   // source:id of the primary install
  "app_id": "google-chrome", "app_name": "Google Chrome",
  "kind": "app",                        // app | os | package
  "source": "mac_app", "software_id": "com.google.Chrome", "name": "Google Chrome",
  "browser": "chrome",                  // or null; from the scanner
  "version_scheme": "dotted",           // dotted | deb | rpm
  "installed": "157.0.8103.30",         // the oldest install's version
  "running": true,
  "installs": [{ "source": "mac_app", "id": "com.google.Chrome", "name": "Google Chrome",
                 "version": "157.0.8103.30", "running": true }],
  "fixed_version": "157.0.8103.40",     // lowest version closing every hole that has a fix, or null
  "latest": "157.0.8103.39",            // from the catalog, or null
  "update_available": true,
  "exploited": true,
  "severity": "critical",               // worst affecting hole, or null when none affects this version
  "vulns": [{ "id": "CVE-2099-0007", "aliases": [], "published": "2026-11-06",
              "severity": "unknown", "cvss": null, "exploited": true,
              "kev": { "added": "2026-11-06", "due": "2026-11-27", "ransomware": false },
              "summary": "...", "fixed": "157.0.8103.40" }]  // fixed: null = no fix yet
}
```

## Matching rules

- `match` rules use the scanner's source names. `windows_registry` matches a
  `DisplayName` prefix. `dpkg` and `rpm` match the **source** package, so all
  binary packages built from it (`openssl`, `libssl3t64`) become installs of
  one finding. `os` matches the device's `os.id` and uses `os.version`.
- An app installed more than once (a cask and its app bundle, say) is one
  finding, judged by its oldest install.
- Range `scope`s apply when they name this device's OS family (`os:macos`) or
  its distro release as an OSV ecosystem (`Debian:12`, `Ubuntu:24.04:LTS`).
- Ranges follow OSV semantics. A hole affected through a range with no fix
  gets `fixed: null`, and doesn't hold back `fixed_version` for the others.
- An install whose version doesn't read in the app's `version_scheme` is
  skipped rather than guessed at.

Versions: `dotted` compares the leading dot-separated numbers; `deb` uses the
[`debversion`](https://crates.io/crates/debversion) crate (dpkg rules); `rpm`
follows rpm's `rpmvercmp`, tested against rpm's own vectors, because no
maintained crate builds on our MSRV.

## Not yet

- **Signature checks.** Catalogs are unsigned until the keys exist. Only
  development builds may accept one; checking `catalog.json.sig` and the key
  delegation goes in front of `Catalog::from_json` then.
- Fedora and other rpm distros have no distro-scoped ranges in the catalog yet.

## Try it

```sh
cargo test -p wardwatch-feed
```

The fixture in `tests/fixtures/catalog.json` is the backend's
`npm run build:fixtures` output.
