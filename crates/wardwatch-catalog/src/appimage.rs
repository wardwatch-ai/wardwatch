//! AppImages: single-file Linux apps that no package database records. They
//! are found where people and AppImage tools keep them (`~/Applications`,
//! `~/.local/bin`, `/opt`, ...) and through the menu entries that
//! AppImageLauncher and appimaged write, which carry the app's version.
//!
//! A file counts only if its header says it is an AppImage (ELF with the
//! `AI` magic), so a stray `.AppImage` name is ignored. The image itself is
//! never mounted or run. While an AppImage runs, its own runtime process keeps
//! the file open as its executable, which is how `running` is set.

use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::{Software, Source, home_dir};

/// Folders, relative to the home folder, that are searched (not recursively).
pub const HOME_DIRS: &[&str] = &[
    "Applications",
    "AppImages",
    ".local/bin",
    "bin",
    "Desktop",
    "Downloads",
];

/// System folders searched (not recursively).
pub const SYSTEM_DIRS: &[&str] = &["/opt", "/usr/local/bin"];

/// Menu entries folder, relative to the home folder.
pub const DESKTOP_DIR: &str = ".local/share/applications";

/// Whether `path` is a regular file whose header marks it as an AppImage:
/// ELF magic, then `AI` and the AppImage type (1 or 2) at offset 8.
pub fn is_appimage(path: &Path) -> bool {
    if !fs::metadata(path).is_ok_and(|m| m.is_file()) {
        return false;
    }
    let mut head = [0u8; 11];
    fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut head))
        .is_ok()
        && head.starts_with(b"\x7fELF")
        && &head[8..10] == b"AI"
        && matches!(head[10], 1 | 2)
}

fn has_appimage_extension(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("appimage"))
}

/// Architecture tags that end AppImage file names.
const ARCH_TAGS: &[&str] = &[
    "x86_64", "x86-64", "amd64", "x64", "aarch64", "arm64", "armhf", "armv7l", "i386", "i686",
    "linux", "linux64",
];

/// The app name and version in an AppImage file name:
/// `Obsidian-1.7.7.AppImage` is `("Obsidian", Some("1.7.7"))`,
/// `LM-Studio-0.3.5-2-x64.AppImage` is `("LM-Studio", Some("0.3.5-2"))`,
/// `nvim.appimage` is `("nvim", None)`.
pub fn parse_file_name(file_name: &str) -> Option<(String, Option<String>)> {
    if !has_appimage_extension(file_name) {
        return None;
    }
    let mut stem = &file_name[..file_name.len() - 9];
    // Drop trailing architecture tags ("-x86_64", "_amd64", "-linux-x64").
    'strip: loop {
        for tag in ARCH_TAGS {
            let cut = stem.len().saturating_sub(tag.len());
            // `get` also checks `cut` is a character boundary; the separator is ASCII.
            if cut > 1
                && stem.get(cut..).is_some_and(|t| t.eq_ignore_ascii_case(tag))
                && stem[..cut].ends_with(['-', '_', '.'])
            {
                stem = &stem[..cut - 1];
                continue 'strip;
            }
        }
        break;
    }
    // The version starts at the first part that starts with a digit (or v1.2).
    let parts: Vec<&str> = stem.split(['-', '_']).collect();
    let starts_version = |p: &&str| {
        let p = p.strip_prefix(['v', 'V']).unwrap_or(p);
        p.starts_with(|c: char| c.is_ascii_digit())
    };
    let at = parts.iter().skip(1).position(starts_version).map(|i| i + 1);
    let (name, version) = match at {
        Some(i) => (parts[..i].join("-"), Some(parts[i..].join("-"))),
        None => (parts.join("-"), None),
    };
    let version = version.map(|v| v.trim_start_matches(['v', 'V']).to_string());
    (!name.is_empty()).then_some((name, version))
}

/// What an AppImage's menu entry says: the file it runs, and the app's name
/// and version (`X-AppImage-Version`) when given.
#[derive(Debug, PartialEq, Eq)]
pub struct DesktopEntry {
    pub exec: PathBuf,
    pub name: Option<String>,
    pub version: Option<String>,
}

/// Parses a `.desktop` file whose `Exec` runs an AppImage by absolute path.
pub fn parse_desktop_entry(text: &str) -> Option<DesktopEntry> {
    let mut in_entry = false;
    let (mut exec, mut name, mut version) = (None, None, None);
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if !in_entry {
            continue;
        }
        let value = value.trim();
        match key.trim() {
            "Exec" if exec.is_none() => {
                let path = match value.strip_prefix('"') {
                    Some(rest) => rest.split('"').next(),
                    None => value.split_whitespace().next(),
                };
                exec = path
                    .filter(|p| p.starts_with('/') && has_appimage_extension(p))
                    .map(PathBuf::from);
            }
            "Name" if name.is_none() && !value.is_empty() => name = Some(value.to_string()),
            "X-AppImage-Version" if !value.is_empty() => version = Some(value.to_string()),
            _ => {}
        }
    }
    Some(DesktopEntry {
        exec: exec?,
        name,
        version,
    })
}

/// AppImages in `dirs` (not recursive) and those that the menu entries in
/// `desktop_dir` run. Each file is listed once, by its real path.
pub fn scan_dirs(dirs: &[PathBuf], desktop_dir: Option<&Path>) -> Vec<Software> {
    let mut found: Vec<(PathBuf, Option<DesktopEntry>)> = Vec::new();
    if let Some(Ok(entries)) = desktop_dir.map(fs::read_dir) {
        for e in entries.filter_map(Result::ok) {
            let path = e.path();
            if path.extension().is_some_and(|x| x == "desktop") {
                let Ok(bytes) = crate::read_capped(&path) else {
                    continue;
                };
                if let Some(entry) = parse_desktop_entry(&String::from_utf8_lossy(&bytes)) {
                    found.push((entry.exec.clone(), Some(entry)));
                }
            }
        }
    }
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for e in entries.filter_map(Result::ok) {
            if e.file_name().to_str().is_some_and(has_appimage_extension) {
                found.push((e.path(), None));
            }
        }
    }

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    // Menu entries come first, so their name and version win over the file name's.
    for (path, entry) in found {
        let Ok(real) = fs::canonicalize(&path) else {
            continue;
        };
        if !seen.insert(real.clone()) || !is_appimage(&real) {
            continue;
        }
        let file_name = real.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // A renamed file may have no AppImage extension; fall back to the link's name.
        let parsed =
            parse_file_name(file_name).or_else(|| parse_file_name(path.file_name()?.to_str()?));
        let Some((file_app, file_version)) = parsed else {
            continue;
        };
        let (name, version) = match entry {
            Some(d) => (d.name.unwrap_or(file_app), d.version.or(file_version)),
            None => (file_app, file_version),
        };
        let id = name
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("-");
        let mut s = Software::new(Source::Appimage, id, name);
        s.version = version;
        s.location = Some(real);
        out.push(s);
    }
    out.sort_by(|a, b| (&a.id, &a.version).cmp(&(&b.id, &b.version)));
    out
}

fn dirs() -> (Vec<PathBuf>, Option<PathBuf>) {
    let home = home_dir();
    let mut dirs: Vec<PathBuf> = home
        .iter()
        .flat_map(|h| HOME_DIRS.iter().map(move |d| h.join(d)))
        .collect();
    dirs.extend(SYSTEM_DIRS.iter().map(PathBuf::from));
    (dirs, home.map(|h| h.join(DESKTOP_DIR)))
}

/// AppImages on this device. Reading them can't fail as a whole, so this
/// never warns; unreadable folders and files are skipped.
pub fn scan(_warnings: &mut Vec<String>) -> Vec<Software> {
    let (dirs, desktop) = dirs();
    scan_dirs(&dirs, desktop.as_deref())
}

/// Folders whose changes mean AppImages were added or removed.
pub fn watch_paths() -> Vec<PathBuf> {
    let (mut dirs, desktop) = dirs();
    // Downloads changes all the time; a new AppImage there is caught on the next scan.
    dirs.retain(|d| !d.ends_with("Downloads"));
    dirs.extend(desktop);
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(file: &str) -> Option<(String, Option<String>)> {
        parse_file_name(file)
    }

    fn pair(n: &str, v: Option<&str>) -> Option<(String, Option<String>)> {
        Some((n.to_string(), v.map(String::from)))
    }

    #[test]
    fn parses_file_names() {
        assert_eq!(
            name("Obsidian-1.7.7.AppImage"),
            pair("Obsidian", Some("1.7.7"))
        );
        assert_eq!(
            name("LM-Studio-0.3.5-2-x64.AppImage"),
            pair("LM-Studio", Some("0.3.5-2"))
        );
        assert_eq!(
            name("balenaEtcher-1.19.25-x64.AppImage"),
            pair("balenaEtcher", Some("1.19.25"))
        );
        assert_eq!(
            name("Cursor-0.42.3-x86_64.AppImage"),
            pair("Cursor", Some("0.42.3"))
        );
        assert_eq!(
            name("joplin_v3.1.24_amd64.appimage"),
            pair("joplin", Some("3.1.24"))
        );
        assert_eq!(name("nvim.appimage"), pair("nvim", None));
        assert_eq!(name("Signal-linux-x86_64.AppImage"), pair("Signal", None));
        assert_eq!(name("7zip-24.08.AppImage"), pair("7zip", Some("24.08")));
        assert_eq!(name("notes.txt"), None);
        assert_eq!(name(".AppImage"), None);
    }

    #[test]
    fn parses_desktop_entries() {
        let text = "[Desktop Entry]\nName=Obsidian\n\
            Exec=\"/home/u/Applications/Obsidian-1.7.7.AppImage\" %U\n\
            X-AppImage-Version=1.7.7\n[Desktop Action new]\nExec=/usr/bin/other\n";
        assert_eq!(
            parse_desktop_entry(text),
            Some(DesktopEntry {
                exec: "/home/u/Applications/Obsidian-1.7.7.AppImage".into(),
                name: Some("Obsidian".into()),
                version: Some("1.7.7".into()),
            })
        );
        assert_eq!(
            parse_desktop_entry("[Desktop Entry]\nName=Files\nExec=nautilus %U\n"),
            None
        );
    }

    #[cfg(unix)]
    fn write_appimage(path: &Path) {
        let mut bytes = b"\x7fELF\x02\x01\x01\x00AI\x02".to_vec();
        bytes.resize(64, 0);
        fs::write(path, bytes).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn scans_folders_and_menu_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let apps = tmp.path().join("Applications");
        let menu = tmp.path().join("menu");
        let elsewhere = tmp.path().join("elsewhere");
        for d in [&apps, &menu, &elsewhere] {
            fs::create_dir_all(d).unwrap();
        }
        write_appimage(&apps.join("Obsidian-1.7.7.AppImage"));
        write_appimage(&elsewhere.join("cursor_ab12cd.AppImage"));
        // Named like an AppImage but isn't one.
        fs::write(apps.join("fake-1.0.AppImage"), b"#!/bin/sh\n").unwrap();
        std::os::unix::fs::symlink(
            apps.join("Obsidian-1.7.7.AppImage"),
            apps.join("obsidian.AppImage"),
        )
        .unwrap();
        fs::write(
            menu.join("appimagekit_x-Cursor.desktop"),
            format!(
                "[Desktop Entry]\nName=Cursor\nExec={} --no-sandbox %F\nX-AppImage-Version=0.42.3\n",
                elsewhere.join("cursor_ab12cd.AppImage").display()
            ),
        )
        .unwrap();

        let found = scan_dirs(&[apps.clone(), tmp.path().join("missing")], Some(&menu));
        let got: Vec<(&str, &str, Option<&str>)> = found
            .iter()
            .map(|s| (s.id.as_str(), s.name.as_str(), s.version.as_deref()))
            .collect();
        assert_eq!(
            got,
            [
                ("cursor", "Cursor", Some("0.42.3")),
                ("obsidian", "Obsidian", Some("1.7.7")),
            ]
        );
        assert_eq!(
            found[1].location,
            Some(fs::canonicalize(apps.join("Obsidian-1.7.7.AppImage")).unwrap())
        );
    }
}
