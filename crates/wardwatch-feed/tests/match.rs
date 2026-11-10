//! Matching against `tests/fixtures/catalog.json`, which is the backend's
//! `npm run build:fixtures` output (schema_version 1, expires 2026-11-15).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use wardwatch_catalog::{Browser, Inventory, OsFamily, OsInfo, Software, Source};
use wardwatch_feed::{AppKind, Catalog, CatalogError, Finding, Severity, match_inventory};

const FIXTURE: &[u8] = include_bytes!("fixtures/catalog.json");

/// 2026-11-08T12:00:00Z, inside the fixture's validity window.
fn now() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_794_139_200)
}

fn catalog() -> Catalog {
    Catalog::from_json(FIXTURE, now()).expect("fixture catalog parses")
}

fn os(family: OsFamily, id: &str, version: &str) -> OsInfo {
    OsInfo {
        family,
        id: id.into(),
        name: id.into(),
        version: version.into(),
        build: None,
        release: None,
        kernel: None,
        arch: "aarch64".into(),
    }
}

fn sw(source: Source, id: &str, name: &str, version: &str) -> Software {
    Software {
        source,
        id: id.into(),
        name: name.into(),
        version: Some(version.into()),
        publisher: None,
        arch: None,
        source_package: None,
        browser: None,
        running: false,
        location: None,
    }
}

fn inventory(os: OsInfo, software: Vec<Software>) -> Inventory {
    Inventory {
        schema: wardwatch_catalog::SCHEMA_VERSION,
        os,
        software,
        services: vec![],
        extensions: vec![],
        warnings: vec![],
    }
}

fn ids(f: &Finding) -> Vec<&str> {
    f.vulns.iter().map(|v| v.id.as_str()).collect()
}

#[test]
fn mac_chrome_behind_an_exploited_fix() {
    let mut chrome = sw(
        Source::MacApp,
        "com.google.Chrome",
        "Google Chrome",
        "157.0.8103.30",
    );
    chrome.browser = Some(Browser::Chrome);
    chrome.running = true;
    let inv = inventory(os(OsFamily::Macos, "macos", "16.1.1"), vec![chrome]);

    let found = match_inventory(&inv, &catalog());
    assert_eq!(found.len(), 1);
    let f = &found[0];
    assert_eq!(f.key, "google-chrome");
    assert_eq!(
        (f.source.as_str(), f.software_id.as_str()),
        ("mac_app", "com.google.Chrome")
    );
    assert_eq!(f.app_id, "google-chrome");
    assert_eq!(f.kind, AppKind::App);
    assert_eq!(f.browser, Some(Browser::Chrome));
    assert!(f.running);
    assert_eq!(ids(f), ["CVE-2099-0007", "CVE-2026-121257"]);
    assert!(f.exploited);
    assert_eq!(f.severity, Some(Severity::Critical));
    // The exploited fix is newer than the catalog's `latest`; the fix wins.
    assert_eq!(f.fixed_version.as_deref(), Some("157.0.8103.40"));
    assert_eq!(f.latest.as_deref(), Some("157.0.8103.39"));
    assert!(f.update_available);
}

#[test]
fn per_os_ranges_and_last_affected() {
    let chrome = sw(
        Source::WindowsRegistry,
        "{8A69D345-D564-463C-AFF1-A69D9E530F96}",
        "Google Chrome",
        "156.0.8000.5",
    );
    let windows = inventory(
        os(OsFamily::Windows, "windows", "10.0.26100.1"),
        vec![chrome.clone()],
    );
    let f = &match_inventory(&windows, &catalog())[0];
    assert!(ids(f).contains(&"CVE-2099-0004"), "{:?}", ids(f));
    let fix = f.vulns.iter().find(|v| v.id == "CVE-2099-0004").unwrap();
    assert_eq!(fix.fixed, None, "last_affected only: no fix in that range");

    // The same registry entry isn't Windows-scoped on another OS (sanity: scope filters).
    let mac = inventory(os(OsFamily::Macos, "macos", "16.1"), vec![chrome]);
    assert!(
        match_inventory(&mac, &catalog())[0]
            .vulns
            .iter()
            .all(|v| v.id != "CVE-2099-0004")
    );
}

#[test]
fn rules_match_only_their_own_source() {
    let win = os(OsFamily::Windows, "windows", "10.0.1");
    // The registry rule is a DisplayName prefix, so "Chrome" alone isn't it.
    let unnamed = sw(Source::WindowsRegistry, "x", "Chrome", "1.0");
    assert!(match_inventory(&inventory(win.clone(), vec![unnamed]), &catalog()).is_empty());
    // A bundle ID only matches the mac_app rule, never the registry one.
    let snap = sw(Source::Snap, "com.google.Chrome", "Google Chrome", "1.0");
    assert!(match_inventory(&inventory(win, vec![snap]), &catalog()).is_empty());
}

#[test]
fn up_to_date_chrome_has_nothing_to_do() {
    let chrome = sw(
        Source::MacApp,
        "com.google.Chrome",
        "Google Chrome",
        "157.0.8103.40",
    );
    let inv = inventory(os(OsFamily::Macos, "macos", "16.1"), vec![chrome]);
    let f = &match_inventory(&inv, &catalog())[0];
    assert!(f.vulns.is_empty());
    assert_eq!(f.severity, None);
    assert!(!f.exploited);
    assert_eq!(f.fixed_version, None);
    assert!(!f.update_available, "latest (.39) is older than installed");
}

#[test]
fn debian_packages_group_by_source_package() {
    let openssl = sw(Source::Dpkg, "openssl", "openssl", "3.0.20-1");
    let mut libssl = sw(Source::Dpkg, "libssl3", "libssl3", "3.0.19-1");
    libssl.source_package = Some("openssl".into());
    let unrelated = sw(Source::Dpkg, "curl", "curl", "8.0-1");
    let inv = inventory(
        os(OsFamily::Linux, "debian", "12"),
        vec![openssl, libssl, unrelated],
    );

    let found = match_inventory(&inv, &catalog());
    assert_eq!(found.len(), 1);
    let f = &found[0];
    assert_eq!(f.kind, AppKind::Package);
    assert_eq!(f.key, "openssl-linux");
    assert_eq!(f.software_id, "libssl3", "the oldest install is primary");
    assert_eq!(f.installed, "3.0.19-1");
    let names: Vec<_> = f.installs.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(names, ["libssl3", "openssl"]);
    assert!(f.exploited, "CVE-2099-1005 is fixed in 3.0.20-1");
    assert_eq!(f.severity, Some(Severity::Critical));
    // 3.0.22-1 sorts after 3.0.22-1~deb12u2, so it's the version that closes everything.
    assert_eq!(f.fixed_version.as_deref(), Some("3.0.22-1"));
    let unfixed = f.vulns.iter().find(|v| v.id == "CVE-2099-1002").unwrap();
    assert_eq!(unfixed.fixed, None);
    assert!(f.update_available);
}

#[test]
fn distro_scopes_only_apply_to_their_release() {
    let openssl = sw(Source::Dpkg, "openssl", "openssl", "3.0.1-1");
    let inv = inventory(os(OsFamily::Linux, "ubuntu", "24.04"), vec![openssl]);
    let f = &match_inventory(&inv, &catalog())[0];
    assert!(f.vulns.is_empty(), "every fixture range is Debian:12");
    assert!(!f.update_available);
}

#[test]
fn unreadable_versions_are_skipped() {
    let chrome = sw(
        Source::MacApp,
        "com.google.Chrome",
        "Google Chrome",
        "unknown",
    );
    let inv = inventory(os(OsFamily::Macos, "macos", "16.1"), vec![chrome]);
    assert!(match_inventory(&inv, &catalog()).is_empty());
}

#[test]
fn most_urgent_first() {
    let chrome = sw(
        Source::MacApp,
        "com.google.Chrome",
        "Google Chrome",
        "157.0.8103.40",
    );
    let openssl = sw(Source::Dpkg, "openssl", "openssl", "3.0.1-1");
    let inv = inventory(os(OsFamily::Linux, "debian", "12"), vec![chrome, openssl]);
    let found = match_inventory(&inv, &catalog());
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].app_id, "openssl-linux");
    assert_eq!(found[1].app_id, "google-chrome");
}

#[test]
fn findings_serialize_in_the_documented_shape() {
    let chrome = sw(
        Source::MacApp,
        "com.google.Chrome",
        "Google Chrome",
        "157.0.8103.30",
    );
    let inv = inventory(os(OsFamily::Macos, "macos", "16.1"), vec![chrome]);
    let json = serde_json::to_value(&match_inventory(&inv, &catalog())[0]).unwrap();
    assert_eq!(json["key"], "google-chrome");
    assert_eq!(json["kind"], "app");
    assert_eq!(json["severity"], "critical");
    assert_eq!(json["version_scheme"], "dotted");
    assert_eq!(json["vulns"][0]["kev"]["ransomware"], false);
    assert_eq!(json["browser"], serde_json::Value::Null);
}

#[test]
fn refuses_expired_wrong_schema_and_garbage() {
    let later = now() + Duration::from_secs(30 * 86_400);
    assert!(matches!(
        Catalog::from_json(FIXTURE, later),
        Err(CatalogError::Expired { .. })
    ));
    let v2 = String::from_utf8(FIXTURE.to_vec())
        .unwrap()
        .replacen("\"schema_version\": 1", "\"schema_version\": 2", 1)
        .replacen("\"schema_version\":1", "\"schema_version\":2", 1);
    assert_eq!(
        Catalog::from_json(v2.as_bytes(), now()),
        Err(CatalogError::Schema(2))
    );
    assert!(matches!(
        Catalog::from_json(b"{}", now()),
        Err(CatalogError::Parse(_))
    ));
}

#[test]
fn a_stale_second_copy_of_an_app_decides() {
    let new = sw(
        Source::MacApp,
        "com.google.Chrome",
        "Google Chrome",
        "157.0.8103.40",
    );
    let old = sw(
        Source::MacApp,
        "com.google.Chrome",
        "Google Chrome",
        "150.0.1.1",
    );
    let inv = inventory(os(OsFamily::Macos, "macos", "16.1"), vec![new, old]);
    let f = &match_inventory(&inv, &catalog())[0];
    assert_eq!(f.installed, "150.0.1.1");
    assert_eq!(f.installs.len(), 2);
    assert!(f.update_available);
}

#[test]
fn unreadable_catalog_bounds_fail_closed() {
    let json = String::from_utf8(FIXTURE.to_vec())
        .unwrap()
        .replace("\"157.0.8103.40\"", "\"v157-beta\"");
    let catalog = Catalog::from_json(json.as_bytes(), now()).unwrap();
    let chrome = sw(
        Source::MacApp,
        "com.google.Chrome",
        "Google Chrome",
        "157.0.8103.39",
    );
    let inv = inventory(os(OsFamily::Macos, "macos", "16.1"), vec![chrome]);
    let f = &match_inventory(&inv, &catalog)[0];
    let hit = f
        .vulns
        .iter()
        .find(|v| v.id == "CVE-2099-0007")
        .expect("still reported");
    assert_eq!(hit.fixed, None, "no readable fix version to point to");
    assert!(f.exploited);
}

#[test]
fn registry_rules_may_name_the_uninstall_key() {
    let json = String::from_utf8(FIXTURE.to_vec()).unwrap().replace(
        "\"name_prefix\":\"Google Chrome\"",
        "\"id\":\"{8A69D345-D564-463C-AFF1-A69D9E530F96}\"",
    );
    let catalog = Catalog::from_json(json.as_bytes(), now()).unwrap();
    let chrome = sw(
        Source::WindowsRegistry,
        "{8A69D345-D564-463C-AFF1-A69D9E530F96}",
        "Chrome",
        "150.0",
    );
    let inv = inventory(os(OsFamily::Windows, "windows", "10.0.1"), vec![chrome]);
    assert_eq!(match_inventory(&inv, &catalog).len(), 1);
}

/// A catalog naming curl and GIMP the way `wardwatch-ai/backend`
/// `catalog/data/apps.json` does for MacPorts, AppImage and winget installs.
fn new_sources_catalog() -> Catalog {
    let json = r#"{
      "schema_version": 1,
      "generated_at": "2026-11-08T00:00:00Z",
      "expires_at": "2026-11-15T00:00:00Z",
      "sources": {},
      "apps": [
        {
          "id": "curl", "name": "curl", "kind": "package", "version_scheme": "dotted",
          "match": [
            {"source": "homebrew_formula", "id": "curl"},
            {"source": "macports", "id": "curl"},
            {"source": "winget", "id": "cURL.cURL"}
          ],
          "vulns": [{
            "id": "CVE-2099-1000", "published": "2026-11-01T00:00:00Z",
            "severity": "high", "exploited": false,
            "ranges": [{"introduced": "8.0.0", "fixed": "8.11.1"}],
            "summary": "test"
          }]
        },
        {
          "id": "gimp", "name": "GIMP", "kind": "app", "version_scheme": "dotted",
          "match": [
            {"source": "appimage", "id": "gimp"},
            {"source": "appimage", "id": "gnu-image-manipulation-program"}
          ],
          "vulns": [{
            "id": "CVE-2099-2000", "published": "2026-11-01T00:00:00Z",
            "severity": "medium", "exploited": false,
            "ranges": [{"introduced": "0", "fixed": "3.2.8"}],
            "summary": "test"
          }]
        }
      ]
    }"#;
    Catalog::from_json(json.as_bytes(), now()).unwrap()
}

#[test]
fn macports_versions_read_without_their_revision() {
    // The scanner's own parser, so the id and version are what a Mac reports.
    let ports = wardwatch_catalog::macports::parse_installed(
        "  curl @8.11.0_0+ssl (active)\n  zlib @1.3.1_0 (active)\n",
    );
    assert_eq!(ports[0].version.as_deref(), Some("8.11.0_0"));
    let inv = inventory(os(OsFamily::Macos, "macos", "16.1.1"), ports);
    let found = match_inventory(&inv, &new_sources_catalog());
    assert_eq!(found.len(), 1);
    let f = &found[0];
    assert_eq!((f.key.as_str(), f.source.as_str()), ("curl", "macports"));
    assert_eq!(f.installed, "8.11.0_0");
    assert_eq!(f.fixed_version.as_deref(), Some("8.11.1"));
    assert!(f.update_available);

    let fixed = vec![sw(Source::Macports, "curl", "curl", "8.11.1_2")];
    let inv = inventory(os(OsFamily::Macos, "macos", "16.1.1"), fixed);
    let found = match_inventory(&inv, &new_sources_catalog());
    assert!(found[0].vulns.is_empty() && !found[0].update_available);
}

#[test]
fn winget_portables_match_by_package_id_only() {
    let portable = sw(Source::Winget, "cURL.cURL", "curl", "8.10.1");
    let inv = inventory(os(OsFamily::Windows, "windows", "10.0.1"), vec![portable]);
    let found = match_inventory(&inv, &new_sources_catalog());
    assert_eq!(found.len(), 1);
    assert_eq!(
        (found[0].source.as_str(), found[0].software_id.as_str()),
        ("winget", "cURL.cURL")
    );
    assert_eq!(ids(&found[0]), ["CVE-2099-1000"]);

    // The same id under another source, or another casing, isn't this rule.
    let others = vec![
        sw(Source::WindowsRegistry, "cURL.cURL", "curl", "8.10.1"),
        sw(Source::Winget, "curl.curl", "curl", "8.10.1"),
    ];
    let inv = inventory(os(OsFamily::Windows, "windows", "10.0.1"), others);
    assert!(match_inventory(&inv, &new_sources_catalog()).is_empty());
}

#[test]
fn appimages_match_by_file_or_menu_name() {
    use wardwatch_catalog::appimage::parse_file_name;
    // How the scanner names an AppImage: lowercased, spaces as hyphens.
    let (name, version) = parse_file_name("GIMP-3.2.6-x86_64.AppImage").unwrap();
    assert_eq!((name.as_str(), version.as_deref()), ("GIMP", Some("3.2.6")));
    let software = vec![
        sw(Source::Appimage, &name.to_lowercase(), &name, "3.2.6"),
        // AppImageLauncher's menu entry carries the long name.
        sw(
            Source::Appimage,
            "gnu-image-manipulation-program",
            "GNU Image Manipulation Program",
            "3.2.4",
        ),
    ];
    let inv = inventory(os(OsFamily::Linux, "fedora", "41"), software);
    let found = match_inventory(&inv, &new_sources_catalog());
    assert_eq!(found.len(), 1);
    let f = &found[0];
    assert_eq!((f.key.as_str(), f.source.as_str()), ("gimp", "appimage"));
    // The older copy decides, and both are listed.
    assert_eq!(f.installed, "3.2.4");
    assert_eq!(f.installs.len(), 2);
    assert_eq!(f.fixed_version.as_deref(), Some("3.2.8"));
}
