//! Linux: the distro from `os-release`, and native packages from dpkg, rpm,
//! snap and Flatpak. dpkg and snap are read straight from disk; rpm and
//! Flatpak have no stable on-disk format, so their own tools are run by
//! absolute path.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::{OsFamily, OsInfo, Software, Source};

/// Parses an `os-release` file (`KEY=value`, values optionally quoted).
pub fn parse_os_release(text: &str) -> Result<OsInfo, String> {
    let fields: BTreeMap<&str, String> = text
        .lines()
        .filter_map(|l| l.trim().split_once('='))
        .filter(|(k, _)| !k.starts_with('#'))
        .map(|(k, v)| (k.trim(), unquote(v.trim())))
        .collect();
    let get = |k: &str| fields.get(k).filter(|v| !v.is_empty()).cloned();
    let id = get("ID").unwrap_or_else(|| "linux".into());
    Ok(OsInfo {
        family: OsFamily::Linux,
        name: get("PRETTY_NAME")
            .or_else(|| get("NAME"))
            .unwrap_or_else(|| "Linux".into()),
        // Rolling distros (Arch) have no VERSION_ID; BUILD_ID is the closest.
        version: get("VERSION_ID")
            .or_else(|| get("BUILD_ID"))
            .unwrap_or_default(),
        build: None,
        release: get("VERSION_CODENAME"),
        kernel: None,
        arch: std::env::consts::ARCH.into(),
        id,
    })
}

fn unquote(v: &str) -> String {
    let inner = v
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| v.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')));
    match inner {
        Some(s) => s
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
            .replace("\\$", "$")
            .replace("\\`", "`"),
        None => v.to_string(),
    }
}

pub fn os_info() -> Result<OsInfo, String> {
    let text = fs::read_to_string("/etc/os-release")
        .or_else(|_| fs::read_to_string("/usr/lib/os-release"))
        .map_err(|_| "os-release not found".to_string())?;
    let mut os = parse_os_release(&text)?;
    os.kernel = sysinfo::System::kernel_version();
    Ok(os)
}

/// Parses dpkg's `/var/lib/dpkg/status`, keeping installed packages
/// (including ones only waiting on triggers, whose files are in place).
pub fn parse_dpkg_status(text: &str) -> Vec<Software> {
    let mut out = Vec::new();
    let mut f: BTreeMap<&str, &str> = BTreeMap::new();
    // A blank line ends a stanza; the trailing "" handles a file with no final blank line.
    for line in text.lines().chain([""]) {
        if line.trim().is_empty() {
            if let Some(s) = dpkg_entry(&f) {
                out.push(s);
            }
            f.clear();
        } else if !line.starts_with([' ', '\t']) {
            // Continuation lines (descriptions, conffiles) start with a space.
            if let Some((k, v)) = line.split_once(':') {
                f.insert(k, v.trim());
            }
        }
    }
    out
}

fn dpkg_entry(f: &BTreeMap<&str, &str>) -> Option<Software> {
    let pkg = *f.get("Package")?;
    let state = f.get("Status")?.split_whitespace().nth(2)?;
    if !matches!(state, "installed" | "triggers-awaited" | "triggers-pending") {
        return None;
    }
    let mut s = Software::new(Source::Dpkg, pkg, pkg);
    s.version = f.get("Version").map(|v| v.to_string());
    s.arch = f.get("Architecture").map(|a| a.to_string());
    // "Source: openssl (3.0.13-0ubuntu3)" names the source and, if different, its version.
    s.source_package = f
        .get("Source")
        .and_then(|src| src.split_whitespace().next())
        .filter(|src| *src != pkg)
        .map(String::from);
    Some(s)
}

/// `rpm --queryformat` for [`parse_rpm_output`]: name, [epoch:]version-release,
/// arch, source rpm, vendor.
pub const RPM_QUERY_FORMAT: &str = "%{NAME}\\t%|EPOCH?{%{EPOCH}:}:{}|%{VERSION}-%{RELEASE}\\t%{ARCH}\\t%{SOURCERPM}\\t%{VENDOR}\\n";

/// Parses `rpm -qa --queryformat RPM_QUERY_FORMAT` output.
pub fn parse_rpm_output(text: &str) -> Vec<Software> {
    let none = |v: &str| (!v.is_empty() && v != "(none)").then(|| v.to_string());
    let mut out = Vec::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let [name, version, arch, srpm, vendor] = cols[..] else {
            continue;
        };
        // Imported signing keys show up as packages.
        if name == "gpg-pubkey" || name.is_empty() {
            continue;
        }
        let mut s = Software::new(Source::Rpm, name, name);
        s.version = none(version);
        s.arch = none(arch);
        s.publisher = none(vendor);
        // "openssl-3.2.2-1.fc40.src.rpm" -> "openssl"
        s.source_package = srpm
            .strip_suffix(".src.rpm")
            .or_else(|| srpm.strip_suffix(".nosrc.rpm"))
            .and_then(|nvr| nvr.rsplitn(3, '-').nth(2))
            .filter(|src| *src != name)
            .map(String::from);
        out.push(s);
    }
    out
}

/// `flatpak list` columns for [`parse_flatpak_output`].
pub const FLATPAK_COLUMNS: &str = "application,version,branch,arch";

/// Parses `flatpak list --columns=application,version,branch,arch`. Apps and
/// runtimes both count: an outdated runtime is as exposed as an outdated app.
/// Runtimes often have no version, only a branch (`47`, `24.08`), so the
/// branch stands in for the version then.
pub fn parse_flatpak_output(text: &str) -> Vec<Software> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for line in text.lines() {
        let mut cols = line.split('\t').map(str::trim);
        let mut next = || cols.next().filter(|c| !c.is_empty()).map(String::from);
        let (Some(id), version, branch, arch) = (next(), next(), next(), next()) else {
            continue;
        };
        if id == "Application ID" {
            continue;
        }
        if !seen.insert((id.clone(), version.clone(), branch.clone())) {
            continue;
        }
        let version = version.or(branch);
        let mut s = Software::new(Source::Flatpak, &id, &id);
        s.version = version;
        s.arch = arch;
        out.push(s);
    }
    out
}

/// Reads the `version:` line of a snap's `meta/snap.yaml`.
pub fn parse_snap_version(yaml: &str) -> Option<String> {
    yaml.lines()
        .find_map(|l| l.strip_prefix("version:"))
        .map(|v| unquote(v.trim()))
        .filter(|v| !v.is_empty())
}

/// Snap mount folders: `/snap` on Debian and Ubuntu, `/var/lib/snapd/snap`
/// on Fedora, openSUSE and Arch (where `/snap` is at most a symlink to it).
pub const SNAP_DIRS: [&str; 2] = ["/snap", "/var/lib/snapd/snap"];

/// Lists snaps under one snap mount folder.
pub fn scan_snaps(snap_dir: &Path) -> Vec<Software> {
    let Ok(entries) = fs::read_dir(snap_dir) else {
        return Vec::new();
    };
    let mut out: Vec<Software> = entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let yaml = crate::read_capped(&e.path().join("current/meta/snap.yaml")).ok()?;
            let yaml = String::from_utf8_lossy(&yaml);
            let mut s = Software::new(Source::Snap, &name, &name);
            s.version = parse_snap_version(&yaml);
            s.location = Some(e.path());
            Some(s)
        })
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// Runs a package tool by absolute path with a fixed locale and returns stdout.
fn run(tool: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(tool)
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| format!("could not run ({})", e.kind()))?;
    if !out.status.success() {
        return Err(format!("exited with {}", out.status));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Scans every native package source present on this machine.
pub fn scan_packages(warnings: &mut Vec<String>) -> Vec<Software> {
    let mut out = Vec::new();

    match fs::read_to_string("/var/lib/dpkg/status") {
        Ok(text) => out.extend(parse_dpkg_status(&text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warnings.push(format!("dpkg: could not read status ({})", e.kind())),
    }

    let rpm_db = ["/var/lib/rpm", "/usr/lib/sysimage/rpm"]
        .iter()
        .any(|p| Path::new(p).is_dir());
    if rpm_db && Path::new("/usr/bin/rpm").exists() {
        match run("/usr/bin/rpm", &["-qa", "--queryformat", RPM_QUERY_FORMAT]) {
            Ok(text) => out.extend(parse_rpm_output(&text)),
            Err(e) => warnings.push(format!("rpm: {e}")),
        }
    }

    let mut snap_ids = HashSet::new();
    for dir in SNAP_DIRS {
        // When /snap links to /var/lib/snapd/snap, keep the first sighting.
        out.extend(
            scan_snaps(Path::new(dir))
                .into_iter()
                .filter(|s| snap_ids.insert(s.id.clone())),
        );
    }

    if Path::new("/usr/bin/flatpak").exists() {
        let columns = format!("--columns={FLATPAK_COLUMNS}");
        match run("/usr/bin/flatpak", &["list", &columns]) {
            Ok(text) => out.extend(parse_flatpak_output(&text)),
            Err(e) => warnings.push(format!("flatpak: {e}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/linux");

    fn fixture(name: &str) -> String {
        fs::read_to_string(Path::new(FIXTURES).join(name)).unwrap()
    }

    #[test]
    fn parses_os_release_files() {
        let ubuntu = parse_os_release(&fixture("os-release-ubuntu")).unwrap();
        assert_eq!(
            (
                ubuntu.id.as_str(),
                ubuntu.name.as_str(),
                ubuntu.version.as_str()
            ),
            ("ubuntu", "Ubuntu 24.04.1 LTS", "24.04")
        );
        assert_eq!(ubuntu.release.as_deref(), Some("noble"));

        let fedora = parse_os_release(&fixture("os-release-fedora")).unwrap();
        assert_eq!(
            (fedora.id.as_str(), fedora.version.as_str()),
            ("fedora", "41")
        );
        assert_eq!(fedora.name, "Fedora Linux 41 (Workstation Edition)");

        let arch = parse_os_release(&fixture("os-release-arch")).unwrap();
        assert_eq!(
            (arch.id.as_str(), arch.version.as_str()),
            ("arch", "rolling")
        );
    }

    #[test]
    fn parses_dpkg_status() {
        let found = parse_dpkg_status(&fixture("dpkg-status"));
        let got: Vec<(&str, Option<&str>, Option<&str>)> = found
            .iter()
            .map(|s| {
                (
                    s.id.as_str(),
                    s.version.as_deref(),
                    s.source_package.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                ("firefox", Some("1:1snap1-0ubuntu5"), None),
                ("google-chrome-stable", Some("131.0.6778.85-1"), None),
                ("libssl3t64", Some("3.0.13-0ubuntu3.4"), Some("openssl")),
                ("openssl", Some("3.0.13-0ubuntu3.4"), None),
                ("tzdata", Some("2024a-3ubuntu1.1"), None),
                ("zlib1g", Some("1:1.3.dfsg-3.1ubuntu2.1"), Some("zlib")),
            ]
        );
        assert_eq!(found[2].arch.as_deref(), Some("amd64"));

        // Same file with Windows line endings (as git may check it out).
        let crlf = fixture("dpkg-status").replace('\n', "\r\n");
        assert_eq!(parse_dpkg_status(&crlf), found);
    }

    #[test]
    fn parses_rpm_output() {
        let found = parse_rpm_output(&fixture("rpm-qa.txt"));
        let got: Vec<(&str, Option<&str>, Option<&str>)> = found
            .iter()
            .map(|s| {
                (
                    s.id.as_str(),
                    s.version.as_deref(),
                    s.source_package.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                ("openssl-libs", Some("1:3.2.2-9.fc41"), Some("openssl")),
                ("firefox", Some("132.0.2-1.fc41"), None),
                ("bash", Some("5.2.32-1.fc41"), None),
                ("kernel-core", Some("6.11.8-300.fc41"), Some("kernel")),
            ]
        );
        assert_eq!(found[0].publisher.as_deref(), Some("Fedora Project"));
        assert_eq!(found[3].publisher, None);
    }

    #[test]
    fn parses_flatpak_output() {
        let found = parse_flatpak_output(&fixture("flatpak-list.txt"));
        let got: Vec<(&str, Option<&str>)> = found
            .iter()
            .map(|s| (s.id.as_str(), s.version.as_deref()))
            .collect();
        assert_eq!(
            got,
            [
                ("org.mozilla.firefox", Some("132.0.2")),
                ("org.freedesktop.Platform", Some("24.08.7")),
                ("org.gnome.Platform", Some("46")),
                ("org.gnome.Platform", Some("47")),
                ("org.example.Nothing", None),
            ]
        );
    }

    #[test]
    fn scans_snaps() {
        let tmp = tempfile::tempdir().unwrap();
        for (name, yaml) in [
            (
                "firefox",
                "name: firefox\nversion: '132.0.2-1'\nsummary: x\n",
            ),
            ("core22", "name: core22\nversion: \"20241001\"\n"),
        ] {
            let meta = tmp.path().join(name).join("current/meta");
            fs::create_dir_all(&meta).unwrap();
            fs::write(meta.join("snap.yaml"), yaml).unwrap();
        }
        fs::create_dir_all(tmp.path().join("bin")).unwrap();
        let found = scan_snaps(tmp.path());
        let got: Vec<(&str, Option<&str>)> = found
            .iter()
            .map(|s| (s.id.as_str(), s.version.as_deref()))
            .collect();
        assert_eq!(
            got,
            [("core22", Some("20241001")), ("firefox", Some("132.0.2-1"))]
        );
        assert_eq!(
            found[1].location.as_deref(),
            Some(tmp.path().join("firefox").as_path())
        );
    }
}
