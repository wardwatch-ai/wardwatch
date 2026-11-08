//! macOS: the OS version from `SystemVersion.plist`, and app bundles from
//! `Info.plist`. Both are plists, read in-process (binary or XML).

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{OsFamily, OsInfo, Software, Source};

#[derive(Deserialize)]
struct SystemVersion {
    #[serde(rename = "ProductName")]
    name: Option<String>,
    #[serde(rename = "ProductVersion")]
    version: String,
    #[serde(rename = "ProductBuildVersion")]
    build: Option<String>,
}

/// Parses `SystemVersion.plist`.
pub fn parse_system_version(bytes: &[u8]) -> Result<OsInfo, String> {
    let v: SystemVersion =
        plist::from_bytes(bytes).map_err(|e| format!("unreadable SystemVersion.plist: {e}"))?;
    Ok(OsInfo {
        family: OsFamily::Macos,
        id: "macos".into(),
        name: v.name.unwrap_or_else(|| "macOS".into()),
        version: v.version,
        build: v.build,
        release: None,
        kernel: None,
        arch: std::env::consts::ARCH.into(),
    })
}

pub fn os_info() -> Result<OsInfo, String> {
    // A Rapid Security Response updates the cryptex copy, not the base one.
    let bytes = [
        "/System/Cryptexes/OS/System/Library/CoreServices/SystemVersion.plist",
        "/System/Library/CoreServices/SystemVersion.plist",
    ]
    .iter()
    .find_map(|p| fs::read(p).ok())
    .ok_or("SystemVersion.plist not found")?;
    let mut os = parse_system_version(&bytes)?;
    os.kernel = sysinfo::System::kernel_version();
    Ok(os)
}

/// Folders apps are installed into: `/Applications` and `~/Applications`.
pub fn app_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/Applications")];
    if let Some(home) = crate::home_dir() {
        dirs.push(home.join("Applications"));
    }
    dirs.retain(|d| d.is_dir());
    dirs
}

/// Reads a plist value as text. Some bundles store versions as `<integer>`
/// or `<real>`, so those count too.
fn text(v: Option<&plist::Value>) -> Option<String> {
    let t = match v? {
        plist::Value::String(s) => s.trim().to_string(),
        plist::Value::Integer(i) => i.to_string(),
        plist::Value::Real(r) => r.to_string(),
        _ => return None,
    };
    (!t.is_empty()).then_some(t)
}

/// Builds an entry from an app's `Info.plist`. `fallback_name` is the bundle's
/// folder name without `.app`.
pub fn parse_info_plist(bytes: &[u8], fallback_name: &str) -> Result<Software, plist::Error> {
    let info: plist::Dictionary = plist::from_bytes(bytes)?;
    let get = |k: &str| text(info.get(k));
    let name = get("CFBundleDisplayName")
        .or_else(|| get("CFBundleName"))
        .unwrap_or_else(|| fallback_name.to_string());
    let id = get("CFBundleIdentifier").unwrap_or_else(|| name.clone());
    let mut s = Software::new(Source::MacApp, id, name);
    s.version = get("CFBundleShortVersionString").or_else(|| get("CFBundleVersion"));
    Ok(s)
}

/// Finds `.app` bundles in `dirs` and one folder level below them
/// (`/Applications/Utilities`, vendor folders). Doesn't look inside bundles.
pub fn scan_apps(dirs: &[PathBuf], warnings: &mut Vec<String>) -> Vec<Software> {
    let mut out = Vec::new();
    let mut unreadable = 0;
    for dir in dirs {
        visit(dir, 0, &mut out, &mut unreadable);
    }
    // A symlinked bundle (e.g. ~/Applications/X.app -> /Applications/X.app)
    // resolves to the same location; keep one entry.
    let mut seen = std::collections::HashSet::new();
    out.retain(|s| seen.insert(s.location.clone()));
    if unreadable > 0 {
        warnings.push(format!(
            "macos: {unreadable} app bundles had no readable Info.plist"
        ));
    }
    out
}

fn visit(dir: &Path, depth: u8, out: &mut Vec<Software>, unreadable: &mut usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| Some(e.ok()?.path())).collect();
    paths.sort();
    for path in paths {
        if path.extension().is_some_and(|e| e == "app") {
            let fallback = path.file_stem().unwrap_or_default().to_string_lossy();
            match crate::read_capped(&path.join("Contents/Info.plist"))
                .ok()
                .and_then(|b| parse_info_plist(&b, &fallback).ok())
            {
                Some(mut s) => {
                    // Running processes report real paths, so store the real path.
                    s.location = Some(fs::canonicalize(&path).unwrap_or(path));
                    out.push(s);
                }
                None => *unreadable += 1,
            }
        } else if depth == 0 && path.is_dir() && !path.is_symlink() {
            visit(&path, depth + 1, out, unreadable);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/macos");

    fn fixture(name: &str) -> Vec<u8> {
        fs::read(Path::new(FIXTURES).join(name)).unwrap()
    }

    #[test]
    fn parses_system_version() {
        let os = parse_system_version(&fixture("SystemVersion.plist")).unwrap();
        assert_eq!(os.family, OsFamily::Macos);
        assert_eq!(os.name, "macOS");
        assert_eq!(os.version, "15.1.1");
        assert_eq!(os.build.as_deref(), Some("24B91"));
    }

    #[test]
    fn parses_xml_and_binary_info_plists() {
        let chrome = parse_info_plist(&fixture("Chrome-Info.plist"), "Google Chrome").unwrap();
        assert_eq!(chrome.id, "com.google.Chrome");
        assert_eq!(chrome.name, "Google Chrome");
        assert_eq!(chrome.version.as_deref(), Some("131.0.6778.86"));

        // Binary plist, as most App Store apps ship. No CFBundleShortVersionString.
        let notes = parse_info_plist(&fixture("Binary-Info.plist"), "Fallback").unwrap();
        assert_eq!(notes.id, "com.example.binary");
        assert_eq!(notes.name, "Binary Example");
        assert_eq!(notes.version.as_deref(), Some("812"));

        // Integer CFBundleVersion and an empty short version, both seen in the wild.
        let int = parse_info_plist(&fixture("Integer-Info.plist"), "Int App").unwrap();
        assert_eq!(
            (int.id.as_str(), int.name.as_str()),
            ("com.example.int", "Int App")
        );
        assert_eq!(int.version.as_deref(), Some("812"));

        assert!(parse_info_plist(b"not a plist", "x").is_err());
    }

    #[test]
    fn scans_app_folders_one_level_deep() {
        let tmp = tempfile::tempdir().unwrap();
        // Locations are canonical (on macOS the temp folder is under /private).
        let root = &fs::canonicalize(tmp.path()).unwrap();
        let chrome = fixture("Chrome-Info.plist");
        for dir in [
            "Google Chrome.app",
            "Utilities/Google Chrome Copy.app",
            "Vendor/Deep/Too Deep.app",
            "Broken.app",
        ] {
            fs::create_dir_all(root.join(dir).join("Contents")).unwrap();
        }
        for dir in [
            "Google Chrome.app",
            "Utilities/Google Chrome Copy.app",
            "Vendor/Deep/Too Deep.app",
        ] {
            fs::write(root.join(dir).join("Contents/Info.plist"), &chrome).unwrap();
        }
        let mut warnings = Vec::new();
        let found = scan_apps(std::slice::from_ref(root), &mut warnings);
        let locations: Vec<_> = found
            .iter()
            .map(|s| {
                s.location
                    .as_ref()
                    .unwrap()
                    .strip_prefix(root)
                    .unwrap()
                    .to_path_buf()
            })
            .collect();
        assert_eq!(
            locations,
            [
                PathBuf::from("Google Chrome.app"),
                PathBuf::from("Utilities/Google Chrome Copy.app")
            ]
        );
        assert_eq!(
            warnings,
            ["macos: 1 app bundles had no readable Info.plist"]
        );
    }
}
