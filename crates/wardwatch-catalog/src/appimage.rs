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

/// What a type 2 AppImage carries about its own updates, read from its ELF
/// sections (the image is never mounted or run).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Embedded {
    /// The `.upd_info` section: where the maker publishes updates, such as
    /// `gh-releases-zsync|owner|repo|latest|App-*x86_64.AppImage.zsync`.
    pub update_info: Option<String>,
    /// Both `.sha256_sig` and `.sig_key` are filled in: the maker signed it,
    /// so AppImageUpdate can insist the new version is signed by the same key.
    pub signed: bool,
}

/// Largest ELF section we read; the AppImage runtime reserves 1 KiB for
/// update info and 1-8 KiB for signatures and keys.
const MAX_SECTION: u64 = 64 << 10;

/// Reads [`Embedded`] from a type 2 AppImage. Unreadable or type 1 images
/// carry nothing we can use, so they read as empty.
pub fn embedded(path: &Path) -> Embedded {
    read_sections(path, &[".upd_info", ".sha256_sig", ".sig_key"])
        .map(|s| {
            let text = |b: &Option<Vec<u8>>| {
                let b = b.as_deref()?;
                let b = &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())];
                std::str::from_utf8(b)
                    .ok()
                    .filter(|t| {
                        !t.is_empty() && t.bytes().all(|c| c.is_ascii_graphic() || c == b' ')
                    })
                    .map(str::to_owned)
            };
            let filled = |b: &Option<Vec<u8>>| {
                b.as_ref()
                    .is_some_and(|b| b.first().is_some_and(|&c| c != 0))
            };
            Embedded {
                update_info: text(&s[0]),
                signed: filled(&s[1]) && filled(&s[2]),
            }
        })
        .unwrap_or_default()
}

/// The contents of the named sections of a little-endian ELF AppImage of
/// type 2, `None` for each one it lacks.
fn read_sections(path: &Path, names: &[&str]) -> std::io::Result<Vec<Option<Vec<u8>>>> {
    use std::io::{Error, Seek, SeekFrom};
    let bad = || Error::other("not a readable type 2 AppImage");
    let mut f = fs::File::open(path)?;
    let mut h = [0u8; 64];
    f.read_exact(&mut h[..52])?;
    if !h.starts_with(b"\x7fELF") || &h[8..11] != b"AI\x02" || h[5] != 1 {
        return Err(bad());
    }
    let wide = match h[4] {
        1 => false,
        2 => true,
        _ => return Err(bad()),
    };
    if wide {
        f.read_exact(&mut h[52..64])?;
    }
    let u16_at = |b: &[u8], o: usize| u64::from(u16::from_le_bytes([b[o], b[o + 1]]));
    let u32_at =
        |b: &[u8], o: usize| u64::from(u32::from_le_bytes(b[o..o + 4].try_into().unwrap()));
    let u64_at = |b: &[u8], o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap());
    // (section table offset, entry size, count, names section index)
    let (shoff, entsize, count, strndx) = if wide {
        (
            u64_at(&h, 0x28),
            u16_at(&h, 0x3a),
            u16_at(&h, 0x3c),
            u16_at(&h, 0x3e),
        )
    } else {
        (
            u32_at(&h, 0x20),
            u16_at(&h, 0x2e),
            u16_at(&h, 0x30),
            u16_at(&h, 0x32),
        )
    };
    let min_entsize = if wide { 0x28 } else { 0x18 };
    if entsize < min_entsize || count == 0 || count > 1024 || strndx >= count {
        return Err(bad());
    }
    let mut table = vec![0u8; usize::try_from(entsize * count).map_err(|_| bad())?];
    f.seek(SeekFrom::Start(shoff))?;
    f.read_exact(&mut table)?;
    // (name offset, file offset, size) of each section.
    let sections: Vec<(u64, u64, u64)> = table
        .chunks_exact(usize::try_from(entsize).map_err(|_| bad())?)
        .map(|e| {
            if wide {
                (u32_at(e, 0), u64_at(e, 0x18), u64_at(e, 0x20))
            } else {
                (u32_at(e, 0), u32_at(e, 0x10), u32_at(e, 0x14))
            }
        })
        .collect();
    let mut read = |offset: u64, size: u64| -> std::io::Result<Vec<u8>> {
        if size > MAX_SECTION {
            return Err(bad());
        }
        let mut buf = vec![0u8; usize::try_from(size).map_err(|_| bad())?];
        f.seek(SeekFrom::Start(offset))?;
        f.read_exact(&mut buf)?;
        Ok(buf)
    };
    let (_, str_off, str_size) = sections[usize::try_from(strndx).map_err(|_| bad())?];
    let strtab = read(str_off, str_size)?;
    let name_of = |at: u64| {
        let rest = strtab.get(usize::try_from(at).ok()?..)?;
        Some(&rest[..rest.iter().position(|&c| c == 0)?])
    };
    names
        .iter()
        .map(|want| {
            match sections
                .iter()
                .find(|(n, _, _)| name_of(*n) == Some(want.as_bytes()))
            {
                Some(&(_, off, size)) => read(off, size).map(Some),
                None => Ok(None),
            }
        })
        .collect()
}

/// A minimal 64-bit type 2 AppImage header with the given ELF sections, for
/// tests here and in the updater.
#[doc(hidden)]
pub fn test_image(sections: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = b"\x7fELF\x02\x01\x01\x00AI\x02".to_vec();
    out.resize(64, 0);
    let mut strtab = vec![0u8];
    // (name offset, data offset, size), with the null section first.
    let mut headers = vec![(0u32, 0u64, 0u64)];
    for (name, data) in sections {
        headers.push((strtab.len() as u32, out.len() as u64, data.len() as u64));
        strtab.extend_from_slice(name.as_bytes());
        strtab.push(0);
        out.extend_from_slice(data);
    }
    let shstrtab_name = strtab.len() as u32;
    strtab.extend_from_slice(b".shstrtab\0");
    headers.push((shstrtab_name, out.len() as u64, strtab.len() as u64));
    out.extend_from_slice(&strtab);
    let shoff = out.len() as u64;
    for (name, offset, size) in &headers {
        let mut e = [0u8; 0x40];
        e[0..4].copy_from_slice(&name.to_le_bytes());
        e[0x18..0x20].copy_from_slice(&offset.to_le_bytes());
        e[0x20..0x28].copy_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&e);
    }
    out[0x28..0x30].copy_from_slice(&shoff.to_le_bytes());
    out[0x3a..0x3c].copy_from_slice(&0x40u16.to_le_bytes());
    out[0x3c..0x3e].copy_from_slice(&(headers.len() as u16).to_le_bytes());
    out[0x3e..0x40].copy_from_slice(&((headers.len() - 1) as u16).to_le_bytes());
    out
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

    #[test]
    fn reads_update_info_and_signature_sections() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("App.AppImage");
        let mut info = b"gh-releases-zsync|maker|app|latest|App-*x86_64.AppImage.zsync".to_vec();
        info.resize(1024, 0);
        let mut unsigned = vec![0u8; 1024];
        fs::write(
            &path,
            test_image(&[
                (".upd_info", &info),
                (".sha256_sig", &unsigned),
                (".sig_key", &unsigned),
            ]),
        )
        .unwrap();
        assert!(is_appimage(&path));
        assert_eq!(
            embedded(&path),
            Embedded {
                update_info: Some(
                    "gh-releases-zsync|maker|app|latest|App-*x86_64.AppImage.zsync".into()
                ),
                signed: false,
            }
        );

        unsigned[0] = b'-';
        let sig = unsigned.clone();
        fs::write(
            &path,
            test_image(&[
                (".upd_info", &[0; 16]),
                (".sha256_sig", &sig),
                (".sig_key", &sig),
            ]),
        )
        .unwrap();
        assert_eq!(
            embedded(&path),
            Embedded {
                update_info: None,
                signed: true
            }
        );

        // A signature without its key isn't one AppImageUpdate can check.
        fs::write(&path, test_image(&[(".sha256_sig", &sig)])).unwrap();
        assert!(!embedded(&path).signed);

        // Not an ELF, a truncated table, a type 1 image: nothing.
        fs::write(&path, b"#!/bin/sh\n").unwrap();
        assert_eq!(embedded(&path), Embedded::default());
        let mut cut = test_image(&[(".sha256_sig", &sig), (".sig_key", &sig)]);
        cut.truncate(cut.len() - 10);
        fs::write(&path, &cut).unwrap();
        assert_eq!(embedded(&path), Embedded::default());
        let mut type1 = test_image(&[(".sha256_sig", &sig), (".sig_key", &sig)]);
        type1[10] = 1;
        fs::write(&path, &type1).unwrap();
        assert_eq!(embedded(&path), Embedded::default());
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
