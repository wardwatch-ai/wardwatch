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
        schema: 1,
        os,
        software,
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
