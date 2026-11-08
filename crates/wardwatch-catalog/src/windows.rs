//! Windows: the OS version and installed programs, both from the registry.
//! Installed programs are the Uninstall keys that "Apps > Installed apps"
//! lists, for the machine (64- and 32-bit views) and for the current user.

use std::collections::{BTreeMap, HashSet};

use crate::{OsFamily, OsInfo, Software, Source};

/// Values of one registry key, as strings (DWORDs in decimal).
pub type Values = BTreeMap<String, String>;

/// Registry values this module reads from each Uninstall subkey.
pub const UNINSTALL_VALUES: &[&str] = &[
    "DisplayName",
    "DisplayVersion",
    "Publisher",
    "InstallLocation",
    "DisplayIcon",
    "SystemComponent",
    "ParentKeyName",
    "ReleaseType",
];

/// Values this module reads from `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion`.
pub const CURRENT_VERSION_VALUES: &[&str] = &[
    "ProductName",
    "DisplayVersion",
    "ReleaseId",
    "CurrentBuildNumber",
    "UBR",
    "CurrentMajorVersionNumber",
    "CurrentMinorVersionNumber",
];

/// Parses `CurrentVersion` values into OS info.
pub fn parse_current_version(v: &Values) -> Result<OsInfo, String> {
    let get = |k: &str| v.get(k).map(|s| s.trim()).filter(|s| !s.is_empty());
    let build: u32 = get("CurrentBuildNumber")
        .and_then(|b| b.parse().ok())
        .ok_or("no CurrentBuildNumber")?;
    let major = get("CurrentMajorVersionNumber").unwrap_or("10");
    let minor = get("CurrentMinorVersionNumber").unwrap_or("0");
    let version = match get("UBR") {
        Some(ubr) => format!("{major}.{minor}.{build}.{ubr}"),
        None => format!("{major}.{minor}.{build}"),
    };
    let mut name = get("ProductName").unwrap_or("Windows").to_string();
    // Windows 11 still reports "Windows 10" in ProductName; build 22000 is 11's first.
    if build >= 22000 {
        name = name.replacen("Windows 10", "Windows 11", 1);
    }
    Ok(OsInfo {
        family: OsFamily::Windows,
        id: "windows".into(),
        name,
        version,
        build: None,
        // DisplayVersion arrived with 20H2; older Windows 10 and Server have ReleaseId.
        release: get("DisplayVersion").or(get("ReleaseId")).map(String::from),
        kernel: None,
        arch: std::env::consts::ARCH.into(),
    })
}

/// Turns Uninstall subkeys `(subkey name, values)` into software entries,
/// dropping what Windows itself hides: system components, updates and patches
/// to other entries, and keys with no name. Drops exact duplicates (a program
/// listed under both registry views).
pub fn parse_uninstall_entries<'a>(
    entries: impl IntoIterator<Item = (&'a str, &'a Values)>,
) -> Vec<Software> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (key, v) in entries {
        let get = |k: &str| v.get(k).map(|s| s.trim()).filter(|s| !s.is_empty());
        let Some(name) = get("DisplayName") else {
            continue;
        };
        if get("SystemComponent") == Some("1")
            || get("ParentKeyName").is_some()
            || get("ReleaseType").is_some_and(|t| {
                matches!(
                    t,
                    "Update" | "Hotfix" | "Security Update" | "Update Rollup" | "Service Pack"
                )
            })
        {
            continue;
        }
        let version = get("DisplayVersion").map(String::from);
        if !seen.insert((name.to_string(), version.clone())) {
            continue;
        }
        let mut s = Software::new(Source::WindowsRegistry, key, name);
        s.version = version;
        s.publisher = get("Publisher").map(String::from);
        s.location = install_dir(get("InstallLocation"), get("DisplayIcon"));
        out.push(s);
    }
    out
}

/// The program's own folder: `InstallLocation`, else the folder of the
/// `DisplayIcon` executable. Drive roots and shared folders such as
/// `C:\Program Files` are rejected, since every program there would match.
fn install_dir(location: Option<&str>, icon: Option<&str>) -> Option<std::path::PathBuf> {
    let from_icon = || {
        // "C:\App\app.exe,0" -> "C:\App"
        let exe = icon?.trim_matches('"');
        let exe = exe
            .rsplit_once(',')
            .map_or(exe, |(p, _)| p)
            .trim_matches('"');
        let dir = exe.rsplit_once('\\')?.0;
        Some(dir.to_string())
    };
    let dir = location
        .map(|l| l.trim_matches('"').trim_end_matches('\\').to_string())
        .filter(|l| !l.is_empty())
        .or_else(from_icon)?;
    // Need at least two folders below the drive: "C:\Vendor\App".
    let depth = dir.split('\\').filter(|p| !p.is_empty()).count();
    let lower = dir.to_lowercase();
    let shared = ["program files", "program files (x86)", "common files"]
        .iter()
        .any(|f| lower.ends_with(&format!("\\{f}")));
    (depth >= 3 && !shared).then(|| dir.into())
}

#[cfg(windows)]
mod sys {
    use winreg::RegKey;
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };

    use super::*;

    const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";

    fn read_values(key: &RegKey, names: &[&str]) -> Values {
        names
            .iter()
            .filter_map(|&n| {
                let v = key
                    .get_value::<String, _>(n)
                    .ok()
                    .or_else(|| key.get_value::<u32, _>(n).ok().map(|d| d.to_string()))?;
                Some((n.to_string(), v))
            })
            .collect()
    }

    pub fn os_info() -> Result<OsInfo, String> {
        let key = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", KEY_READ)
            .map_err(|e| format!("could not open CurrentVersion ({})", e.kind()))?;
        let mut os = parse_current_version(&read_values(&key, CURRENT_VERSION_VALUES))?;
        os.kernel = sysinfo::System::kernel_version();
        Ok(os)
    }

    pub fn scan_registry(warnings: &mut Vec<String>) -> Vec<Software> {
        let views = [
            (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_64KEY),
            (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_32KEY),
            (HKEY_CURRENT_USER, KEY_READ),
        ];
        let mut entries: Vec<(String, Values)> = Vec::new();
        for (hive, flags) in views {
            let Ok(root) = RegKey::predef(hive).open_subkey_with_flags(UNINSTALL, flags) else {
                continue;
            };
            for name in root.enum_keys().filter_map(Result::ok) {
                match root.open_subkey_with_flags(&name, flags) {
                    Ok(k) => {
                        let values = read_values(&k, UNINSTALL_VALUES);
                        entries.push((name, values));
                    }
                    Err(_) => continue,
                }
            }
        }
        if entries.is_empty() {
            warnings.push("windows: no Uninstall registry entries could be read".into());
        }
        parse_uninstall_entries(entries.iter().map(|(k, v)| (k.as_str(), v)))
    }
}

#[cfg(windows)]
pub use sys::{os_info, scan_registry};

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/windows");

    fn fixture<T: serde::de::DeserializeOwned>(name: &str) -> T {
        let path = std::path::Path::new(FIXTURES).join(name);
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn parses_windows_11_current_version() {
        let os = parse_current_version(&fixture("current-version-win11.json")).unwrap();
        assert_eq!(os.name, "Windows 11 Pro");
        assert_eq!(os.version, "10.0.22631.4460");
        assert_eq!(os.release.as_deref(), Some("23H2"));
    }

    #[test]
    fn parses_windows_10_current_version() {
        let os = parse_current_version(&fixture("current-version-win10.json")).unwrap();
        assert_eq!(os.name, "Windows 10 Home");
        assert_eq!(os.version, "10.0.19045.5131");
        assert!(parse_current_version(&Values::new()).is_err());
    }

    #[test]
    fn filters_uninstall_entries_like_windows_does() {
        let entries: Vec<(String, Values)> = fixture("uninstall.json");
        let found = parse_uninstall_entries(entries.iter().map(|(k, v)| (k.as_str(), v)));
        let names: Vec<(&str, Option<&str>)> = found
            .iter()
            .map(|s| (s.name.as_str(), s.version.as_deref()))
            .collect();
        assert_eq!(
            names,
            [
                ("Google Chrome", Some("131.0.6778.86")),
                ("Mozilla Firefox (x64 en-US)", Some("132.0.2")),
                ("Microsoft Edge", Some("131.0.2903.70")),
                ("7-Zip 24.08 (x64)", Some("24.08")),
                ("Notepad++ (64-bit x64)", Some("8.7.1")),
                ("Zoom Workplace", None),
            ]
        );
        assert_eq!(found[0].publisher.as_deref(), Some("Google LLC"));
        assert_eq!(
            found[0].location.as_deref(),
            Some(std::path::Path::new(
                r"C:\Program Files\Google\Chrome\Application"
            ))
        );
        // No InstallLocation: falls back to the DisplayIcon's folder.
        assert_eq!(
            found[4].location.as_deref(),
            Some(std::path::Path::new(r"C:\Program Files\Notepad++"))
        );
    }

    #[test]
    fn rejects_shared_install_folders() {
        assert_eq!(install_dir(Some(r"C:\"), None), None);
        assert_eq!(install_dir(Some(r"C:\Program Files\"), None), None);
        assert_eq!(
            install_dir(Some(r"C:\Program Files\Common Files"), None),
            None
        );
        assert_eq!(install_dir(Some("\"\""), None), None);
        assert_eq!(
            install_dir(Some(r#""C:\Program Files\7-Zip\""#), None),
            Some(r"C:\Program Files\7-Zip".into())
        );
        assert_eq!(
            install_dir(None, Some(r#""C:\Tools\App\app.exe",0"#)),
            Some(r"C:\Tools\App".into())
        );
    }
}
