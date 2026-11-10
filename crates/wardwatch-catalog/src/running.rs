//! Marks installed software that has a running process.

use std::path::{Path, PathBuf};

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::{Browser, Software, Source};

/// Executable paths of running processes. Reads only the exe path of each
/// process (no CPU, memory or command line), which keeps this cheap.
pub(crate) fn running_executables() -> Vec<PathBuf> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
    );
    let mut exes: Vec<PathBuf> = sys
        .processes()
        .values()
        .filter_map(|p| p.exe().map(Path::to_path_buf))
        .collect();
    exes.sort();
    exes.dedup();
    exes
}

/// Executable names of browsers installed from Linux packages, whose
/// executable isn't named after the package.
const BROWSER_EXES: &[(Browser, &[&str])] = &[
    (Browser::Chrome, &["chrome"]),
    (Browser::Chromium, &["chromium", "chromium-browser"]),
    (Browser::Edge, &["msedge"]),
    (Browser::Firefox, &["firefox", "firefox-bin", "firefox-esr"]),
    (Browser::Brave, &["brave"]),
    (Browser::Opera, &["opera"]),
    (Browser::Vivaldi, &["vivaldi-bin"]),
];

pub(crate) fn mark_running(software: &mut [Software], exes: &[PathBuf]) {
    let exe_names: Vec<&str> = exes
        .iter()
        .filter_map(|p| p.file_name()?.to_str())
        .collect();
    let exe_paths: Vec<String> = exes.iter().map(|p| normalize(p)).collect();
    for s in software.iter_mut() {
        s.running = match (&s.location, s.source) {
            // An AppImage's location is the file itself, which its runtime runs from.
            (Some(loc), Source::Appimage) => {
                let file = normalize(loc);
                exe_paths.contains(&file)
            }
            (Some(loc), _) => {
                let dir = normalize(loc);
                exe_paths.iter().any(|exe| inside(&dir, exe))
            }
            // Ports share one prefix, so match by name inside it (not Apple's
            // own /usr/bin/curl).
            (None, Source::Macports) => exe_paths.iter().any(|exe| {
                exe.strip_prefix(crate::macports::PREFIX)
                    .is_some_and(|rest| rest.rsplit('/').next() == Some(s.id.as_str()))
            }),
            // Distro packages have no single install folder, so match by name.
            (None, Source::Dpkg | Source::Rpm) => {
                let browser_exes = s
                    .browser
                    .and_then(|b| BROWSER_EXES.iter().find(|(x, _)| *x == b))
                    .map_or(&[][..], |(_, names)| *names);
                exe_names
                    .iter()
                    .any(|n| *n == s.id || browser_exes.contains(n))
            }
            (None, _) => false,
        };
    }
}

/// Whether normalized `path` is strictly inside normalized folder `dir`.
pub(crate) fn inside(dir: &str, path: &str) -> bool {
    let dir = dir.trim_end_matches('/');
    path.len() > dir.len() && path.starts_with(dir) && path.as_bytes()[dir.len()] == b'/'
}

/// A path as a comparable string: `/` separators, and lowercase on Windows,
/// whose paths are case-insensitive.
pub(crate) fn normalize(p: &Path) -> String {
    let s = p.to_string_lossy();
    if cfg!(windows) {
        s.replace('\\', "/").to_lowercase()
    } else {
        s.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_by_location_and_by_package_name() {
        let mut app = Software::new(Source::MacApp, "com.example.App", "App");
        app.location = Some("/Applications/App.app".into());
        let mut lookalike = Software::new(Source::MacApp, "com.example.AppPro", "App Pro");
        lookalike.location = Some("/Applications/App.app Pro".into());
        let curl = Software::new(Source::Dpkg, "curl", "curl");
        let mut firefox = Software::new(Source::Dpkg, "firefox-esr", "firefox-esr");
        firefox.browser = Some(Browser::Firefox);
        let idle = Software::new(Source::Rpm, "vim-enhanced", "vim-enhanced");
        let flatpak = Software::new(Source::Flatpak, "curl", "curl");
        let mut appimage = Software::new(Source::Appimage, "obsidian", "Obsidian");
        appimage.location = Some("/home/u/Applications/Obsidian-1.7.7.AppImage".into());
        let mut idle_appimage = Software::new(Source::Appimage, "cursor", "Cursor");
        idle_appimage.location = Some("/home/u/Applications/Cursor.AppImage".into());
        let port = Software::new(Source::Macports, "nginx", "nginx");
        let apple_rsync = Software::new(Source::Macports, "rsync", "rsync");

        let mut list = vec![
            app,
            lookalike,
            curl,
            firefox,
            idle,
            flatpak,
            appimage,
            idle_appimage,
            port,
            apple_rsync,
        ];
        let exes: Vec<PathBuf> = [
            "/Applications/App.app/Contents/MacOS/App",
            "/usr/bin/curl",
            "/usr/lib/firefox-esr/firefox-bin",
            "/home/u/Applications/Obsidian-1.7.7.AppImage",
            "/opt/local/sbin/nginx",
            "/usr/bin/rsync",
        ]
        .map(PathBuf::from)
        .to_vec();
        mark_running(&mut list, &exes);
        let running: Vec<bool> = list.iter().map(|s| s.running).collect();
        assert_eq!(
            running,
            [
                true, false, true, true, false, false, true, false, true, false
            ]
        );
    }

    #[test]
    fn running_executables_includes_this_test() {
        let me = std::env::current_exe().unwrap();
        let exes = running_executables();
        assert!(!exes.is_empty());
        let name = me.file_name().unwrap();
        assert!(exes.iter().any(|e| e.file_name() == Some(name)));
    }
}
