//! Finds what is installed on this device: the OS version, browsers, and
//! installed and running software on macOS, Windows and Linux.
//!
//! Everything here runs locally. The [`Inventory`] this crate builds is
//! compared on the device against the signed catalog; it never leaves the
//! device as-is. File paths are kept only in memory (to tell which apps are
//! running) and are never serialized, so usernames in paths can't leak.
//!
//! The scanners read package databases and app metadata directly where they
//! can (dpkg status, `Info.plist`, the Windows registry, Homebrew's Cellar)
//! and only start a subprocess where a database has no stable file format
//! (`rpm`, `flatpak`). A full scan is one pass with no background threads,
//! so callers decide when to rescan; [`watch_paths`] lists what to watch.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

mod browsers;
pub mod homebrew;
pub mod linux;
pub mod macos;
mod running;
pub mod windows;

pub use browsers::Browser;

/// Version of the serialized [`Inventory`] shape.
pub const SCHEMA_VERSION: u32 = 1;

/// Everything found on the device in one scan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    pub schema: u32,
    pub os: OsInfo,
    pub software: Vec<Software>,
    /// Scanners that failed, in words with no paths or usernames.
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OsFamily {
    Macos,
    Windows,
    Linux,
}

/// The operating system, with the identifiers vulnerability feeds use.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsInfo {
    pub family: OsFamily,
    /// `macos`, `windows`, or the Linux `/etc/os-release` `ID` (`ubuntu`, `fedora`, ...).
    pub id: String,
    /// Human name: `macOS`, `Windows 11 Pro`, `Ubuntu 24.04.1 LTS`.
    pub name: String,
    /// Precise version: macOS `15.1.1`, Windows `10.0.22631.4460` (build.UBR),
    /// Linux `VERSION_ID` such as `24.04`.
    pub version: String,
    /// macOS build such as `24B91`, including any Rapid Security Response suffix.
    pub build: Option<String>,
    /// Windows release such as `23H2`, or the Linux `VERSION_CODENAME`.
    pub release: Option<String>,
    pub kernel: Option<String>,
    /// CPU architecture, as Rust names it (`x86_64`, `aarch64`).
    pub arch: String,
}

/// Where a [`Software`] entry was found. Each source has its own `id` namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// macOS app bundle; `id` is `CFBundleIdentifier`.
    MacApp,
    /// Homebrew formula; `id` is the formula name.
    HomebrewFormula,
    /// Homebrew cask; `id` is the cask token.
    HomebrewCask,
    /// Windows Uninstall registry key; `id` is the subkey name (often a GUID),
    /// `name` is `DisplayName`.
    WindowsRegistry,
    /// Debian/Ubuntu package; `id` is the package name.
    Dpkg,
    /// RPM package; `id` is the package `NAME`.
    Rpm,
    /// Snap; `id` is the snap name.
    Snap,
    /// Flatpak app or runtime; `id` is the application ID.
    Flatpak,
}

/// One installed piece of software.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Software {
    pub source: Source,
    pub id: String,
    pub name: String,
    /// The version as the source reports it. dpkg and rpm versions keep their
    /// epoch and revision (`1:3.0.13-0ubuntu3.4`), because distro advisories
    /// are written against exactly those strings.
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    /// Distro source package (dpkg `Source`, rpm `SOURCERPM` name), when it
    /// differs from `id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_package: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<Browser>,
    /// A process running from this software was seen during the scan.
    #[serde(default)]
    pub running: bool,
    /// Install location, used only to match running processes. Never serialized.
    #[serde(skip)]
    pub location: Option<PathBuf>,
}

impl Software {
    pub(crate) fn new(source: Source, id: impl Into<String>, name: impl Into<String>) -> Self {
        Software {
            source,
            id: id.into(),
            name: name.into(),
            version: None,
            publisher: None,
            arch: None,
            source_package: None,
            browser: None,
            running: false,
            location: None,
        }
    }
}

/// Scans this device. Never fails: a scanner that can't run adds a warning.
pub fn scan() -> Inventory {
    let mut warnings = Vec::new();
    let os = current_os(&mut warnings);
    let mut software = Vec::new();

    #[cfg(target_os = "macos")]
    {
        software.extend(macos::scan_apps(&macos::app_dirs(), &mut warnings));
        software.extend(homebrew::scan(&mut warnings));
    }
    #[cfg(windows)]
    software.extend(windows::scan_registry(&mut warnings));
    #[cfg(target_os = "linux")]
    {
        software.extend(linux::scan_packages(&mut warnings));
        software.extend(homebrew::scan(&mut warnings));
    }

    finish(&mut software, &running::running_executables());
    Inventory {
        schema: SCHEMA_VERSION,
        os,
        software,
        warnings,
    }
}

/// Tags browsers, marks running software, and sorts the list so scans diff cleanly.
fn finish(software: &mut [Software], running_exes: &[PathBuf]) {
    for s in software.iter_mut() {
        s.browser = browsers::identify(s);
    }
    running::mark_running(software, running_exes);
    software.sort_by(|a, b| (a.source as u8, &a.id).cmp(&(b.source as u8, &b.id)));
}

fn current_os(warnings: &mut Vec<String>) -> OsInfo {
    #[cfg(target_os = "macos")]
    let os = macos::os_info();
    #[cfg(windows)]
    let os = windows::os_info();
    #[cfg(target_os = "linux")]
    let os = linux::os_info();
    #[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
    let os: Result<OsInfo, String> = Err("unsupported operating system".into());

    os.unwrap_or_else(|e| {
        warnings.push(format!("os: {e}"));
        OsInfo {
            family: if cfg!(windows) {
                OsFamily::Windows
            } else if cfg!(target_os = "macos") {
                OsFamily::Macos
            } else {
                OsFamily::Linux
            },
            id: "unknown".into(),
            name: "Unknown".into(),
            version: String::new(),
            build: None,
            release: None,
            kernel: None,
            arch: std::env::consts::ARCH.into(),
        }
    })
}

/// Files and folders whose changes mean the inventory may have changed, for an
/// event-driven agent to watch instead of polling. Only existing paths are returned.
/// (Windows registry changes are watched with `RegNotifyChangeKeyValue` on the
/// Uninstall keys instead.)
pub fn watch_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();
    #[cfg(target_os = "macos")]
    {
        paths.extend(macos::app_dirs());
        paths.push("/System/Library/CoreServices/SystemVersion.plist".into());
    }
    #[cfg(target_os = "linux")]
    paths.extend(
        [
            "/var/lib/dpkg/status",
            "/var/lib/rpm",
            "/usr/lib/sysimage/rpm",
            "/var/lib/flatpak/app",
            "/snap",
            "/etc/os-release",
        ]
        .map(PathBuf::from),
    );
    #[cfg(unix)]
    for prefix in homebrew::PREFIXES {
        paths.push(std::path::Path::new(prefix).join("Cellar"));
        paths.push(std::path::Path::new(prefix).join("Caskroom"));
    }
    paths.retain(|p| p.exists());
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_tags_sorts_and_hides_paths() {
        let mut chrome = Software::new(Source::MacApp, "com.google.Chrome", "Google Chrome");
        chrome.location = Some("/Applications/Google Chrome.app".into());
        let mut list = vec![Software::new(Source::Dpkg, "zlib1g", "zlib1g"), chrome];
        finish(
            &mut list,
            &["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".into()],
        );
        assert_eq!(list[0].id, "com.google.Chrome");
        assert_eq!(list[0].browser, Some(Browser::Chrome));
        assert!(list[0].running);
        assert!(!list[1].running);

        let json = serde_json::to_string(&list[0]).unwrap();
        assert!(
            !json.contains("/Applications"),
            "paths must not serialize: {json}"
        );
        let back: Software = serde_json::from_str(&json).unwrap();
        assert_eq!(back.location, None);
    }

    #[test]
    fn scan_runs_on_this_machine() {
        let inv = scan();
        assert_eq!(inv.schema, SCHEMA_VERSION);
        assert!(!inv.os.arch.is_empty());
        let json = serde_json::to_string(&inv).unwrap();
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            let home = home.to_string_lossy();
            if home.len() > 1 {
                assert!(!json.contains(home.as_ref()), "home path leaked");
            }
        }
    }
}
