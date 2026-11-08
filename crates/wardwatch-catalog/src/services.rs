//! Background services: systemd units on Linux, launchd jobs on macOS, and
//! Windows services. Services shipped with the OS are left out (Apple's jobs
//! in `/System`, Windows services under the Windows folder), since OS updates
//! cover them; on Linux every unit comes from a package, so all are listed.
//!
//! Each service is linked to the catalogued [`Software`] its program belongs
//! to when the program sits inside that software's install folder (or, for
//! distro packages, is named after the package). Program paths are used only
//! for that and are never serialized.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::running::{inside, normalize};
use crate::{Software, Source};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceKind {
    Systemd,
    Launchd,
    Windows,
}

/// Which catalogued software a service belongs to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoftwareRef {
    pub source: Source,
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Service {
    pub kind: ServiceKind,
    /// Unit name (`ssh.service`), launchd label, or Windows service name.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Starts without anyone opening it: an enabled unit, a launchd job that
    /// runs at load, stays alive or runs on a schedule, an Automatic service.
    pub starts_automatically: bool,
    /// Seen running during the scan. Best effort: a program another user owns
    /// may not be visible.
    pub running: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub software: Option<SoftwareRef>,
    /// The program the service runs. Never serialized.
    #[serde(skip)]
    pub program: Option<PathBuf>,
}

impl Service {
    fn new(kind: ServiceKind, name: impl Into<String>) -> Self {
        Service {
            kind,
            name: name.into(),
            display_name: None,
            starts_automatically: false,
            running: false,
            software: None,
            program: None,
        }
    }
}

/// The software whose install folder holds `program`, else a distro package
/// named like the program's file.
fn owner(software: &[Software], program: &Path) -> Option<SoftwareRef> {
    let path = normalize(program);
    let file = program.file_name()?.to_str()?;
    let by_location = software.iter().find(|s| {
        s.location
            .as_deref()
            .is_some_and(|loc| inside(&normalize(loc), &path))
    });
    let by_name = || {
        software
            .iter()
            .find(|s| matches!(s.source, Source::Dpkg | Source::Rpm) && s.id == file)
    };
    by_location.or_else(by_name).map(|s| SoftwareRef {
        source: s.source,
        id: s.id.clone(),
    })
}

/// Links services to software, marks those whose program is running (unless
/// the scanner already knew), and sorts them.
fn finish(services: &mut Vec<Service>, software: &[Software], running_exes: &[PathBuf]) {
    let running: HashSet<String> = running_exes.iter().map(|p| normalize(p)).collect();
    for svc in services.iter_mut() {
        let Some(program) = &svc.program else {
            continue;
        };
        svc.software = owner(software, program);
        svc.running |= running.contains(&normalize(program));
    }
    services.sort_by(|a, b| (a.kind, &a.name).cmp(&(b.kind, &b.name)));
    services.dedup_by(|a, b| a.kind == b.kind && a.name == b.name);
}

pub fn scan(
    software: &[Software],
    running_exes: &[PathBuf],
    warnings: &mut Vec<String>,
) -> Vec<Service> {
    #[cfg(target_os = "linux")]
    let mut found = {
        let _ = &warnings;
        let dirs: Vec<&Path> = systemd::UNIT_DIRS.iter().map(Path::new).collect();
        systemd::scan_units(
            &dirs,
            Path::new(systemd::ENABLED_ROOT),
            &systemd::running_units(),
        )
    };
    #[cfg(target_os = "macos")]
    let mut found = {
        let _ = &warnings;
        launchd::scan_dirs(&launchd::dirs())
    };
    #[cfg(windows)]
    let mut found = windows::scan(warnings);
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    let mut found: Vec<Service> = {
        let _ = &warnings;
        Vec::new()
    };
    finish(&mut found, software, running_exes);
    found
}

/// Folders whose changes mean services were added or removed.
pub fn watch_paths() -> Vec<PathBuf> {
    #[cfg(target_os = "linux")]
    return vec![
        systemd::ENABLED_ROOT.into(),
        "/usr/lib/systemd/system".into(),
    ];
    #[cfg(target_os = "macos")]
    return launchd::dirs();
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    Vec::new()
}

/// systemd system units, read from unit files; which are running comes from
/// the world-readable `/proc/<pid>/cgroup`, since a user can't read the
/// executable path of root's daemons.
pub mod systemd {
    use std::collections::HashSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{Service, ServiceKind};
    use crate::read_capped;

    /// Unit folders, highest precedence first.
    pub const UNIT_DIRS: &[&str] = &[
        "/etc/systemd/system",
        "/run/systemd/system",
        "/usr/local/lib/systemd/system",
        "/usr/lib/systemd/system",
        "/lib/systemd/system",
    ];

    /// Holds the `*.wants` and `*.requires` folders `systemctl enable` links into.
    pub const ENABLED_ROOT: &str = "/etc/systemd/system";

    /// The `Description` and the program of the first `ExecStart` of a unit file.
    pub fn parse_unit(text: &str) -> (Option<String>, Option<PathBuf>) {
        let mut section = "";
        let mut description = None;
        let mut program = None;
        for line in text.lines().map(str::trim) {
            if line.starts_with('[') {
                section = line;
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match (section, key.trim()) {
                ("[Unit]", "Description") if !value.is_empty() => {
                    description = Some(value.to_string());
                }
                ("[Service]", "ExecStart") if program.is_none() => {
                    // Prefixes such as "-" (ignore failure) and "!" (no privilege drop).
                    let cmd = value.trim_start_matches(['@', '-', ':', '+', '!']);
                    program = cmd
                        .split_whitespace()
                        .next()
                        .filter(|p| p.starts_with('/'))
                        .map(PathBuf::from);
                }
                _ => {}
            }
        }
        (description, program)
    }

    /// The service a process belongs to, from its `/proc/<pid>/cgroup` text.
    pub fn parse_cgroup(text: &str) -> Option<String> {
        text.lines()
            .filter_map(|l| l.rsplit(':').next())
            .flat_map(|path| path.split('/'))
            .rfind(|c| c.ends_with(".service"))
            .map(String::from)
    }

    /// Services with a running process.
    pub fn running_units() -> HashSet<String> {
        let Ok(proc) = fs::read_dir("/proc") else {
            return HashSet::new();
        };
        proc.filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.bytes().all(|b| b.is_ascii_digit()))
            })
            .filter_map(|e| fs::read_to_string(e.path().join("cgroup")).ok())
            .filter_map(|t| parse_cgroup(&t))
            .collect()
    }

    /// Names linked into `*.wants`/`*.requires` folders under `root`.
    fn enabled_units(root: &Path) -> HashSet<String> {
        let Ok(entries) = fs::read_dir(root) else {
            return HashSet::new();
        };
        entries
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.ends_with(".wants") || n.ends_with(".requires"))
            })
            .filter_map(|e| fs::read_dir(e.path()).ok())
            .flatten()
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .collect()
    }

    /// Service units that are enabled or running. Templates (`getty@.service`),
    /// aliases and masked units are skipped.
    pub fn scan_units(
        dirs: &[&Path],
        enabled_root: &Path,
        running: &HashSet<String>,
    ) -> Vec<Service> {
        let enabled = enabled_units(enabled_root);
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for dir in dirs {
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            let mut names: Vec<String> = entries
                .filter_map(|e| e.ok()?.file_name().into_string().ok())
                .filter(|n| n.ends_with(".service") && !n.contains('@'))
                .collect();
            names.sort();
            for name in names {
                let path = dir.join(&name);
                if let Ok(target) = fs::read_link(&path) {
                    if target == Path::new("/dev/null") {
                        seen.insert(name); // masked
                        continue;
                    }
                    if target.file_name().is_some_and(|t| t != name.as_str()) {
                        continue; // alias of another unit
                    }
                }
                if !seen.insert(name.clone()) {
                    continue;
                }
                let is_enabled = enabled.contains(&name);
                let is_running = running.contains(&name);
                if !is_enabled && !is_running {
                    continue;
                }
                let Ok(bytes) = read_capped(&path) else {
                    continue;
                };
                let (description, program) = parse_unit(&String::from_utf8_lossy(&bytes));
                let mut svc = Service::new(ServiceKind::Systemd, name);
                svc.display_name = description;
                svc.starts_automatically = is_enabled;
                svc.running = is_running;
                svc.program = program;
                out.push(svc);
            }
        }
        out
    }
}

/// launchd jobs from the plists in `/Library` and `~/Library`.
pub mod launchd {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{Service, ServiceKind};
    use crate::read_capped;

    /// Third-party job folders. Apple's own are under `/System`.
    pub fn dirs() -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = vec![
            "/Library/LaunchDaemons".into(),
            "/Library/LaunchAgents".into(),
        ];
        if let Some(home) = crate::home_dir() {
            dirs.push(home.join("Library/LaunchAgents"));
        }
        dirs.retain(|d| d.is_dir());
        dirs
    }

    /// Builds a service from a job plist.
    pub fn parse_job(bytes: &[u8]) -> Option<Service> {
        let job: plist::Dictionary = plist::from_bytes(bytes).ok()?;
        let label = job.get("Label")?.as_string()?.trim();
        if label.is_empty() {
            return None;
        }
        let truthy = |k: &str| match job.get(k) {
            Some(plist::Value::Boolean(b)) => *b,
            Some(
                plist::Value::Dictionary(_) | plist::Value::Array(_) | plist::Value::Integer(_),
            ) => true,
            _ => false,
        };
        let program = job
            .get("Program")
            .and_then(plist::Value::as_string)
            .or_else(|| {
                job.get("ProgramArguments")?
                    .as_array()?
                    .first()?
                    .as_string()
            })
            .filter(|p| p.starts_with('/'))
            .map(PathBuf::from);
        let mut svc = Service::new(ServiceKind::Launchd, label);
        svc.starts_automatically = !truthy("Disabled")
            && [
                "RunAtLoad",
                "KeepAlive",
                "StartInterval",
                "StartCalendarInterval",
            ]
            .iter()
            .any(|k| truthy(k));
        // Homebrew jobs run /opt/homebrew/opt/<name>/…, a link into the Cellar.
        svc.program = program.map(|p| fs::canonicalize(&p).unwrap_or(p));
        Some(svc)
    }

    pub fn scan_dirs(dirs: &[PathBuf]) -> Vec<Service> {
        let mut out = Vec::new();
        for dir in dirs {
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            for e in entries.filter_map(Result::ok) {
                let path = e.path();
                if path.extension().is_some_and(|x| x == "plist") {
                    out.extend(
                        read_capped(Path::new(&path))
                            .ok()
                            .and_then(|b| parse_job(&b)),
                    );
                }
            }
        }
        out
    }
}

/// Windows services from `HKLM\SYSTEM\CurrentControlSet\Services`.
pub mod windows {
    use std::path::PathBuf;

    use super::{Service, ServiceKind};
    use crate::windows::Values;

    /// Values read from each service's key.
    pub const SERVICE_VALUES: &[&str] = &["Type", "Start", "ImagePath", "DisplayName"];

    /// Replaces `%NAME%` with `lookup(NAME)`, leaving unknown names as they are.
    pub fn expand_env(s: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
        let mut out = String::new();
        let mut rest = s;
        while let Some(start) = rest.find('%') {
            let Some(len) = rest[start + 1..].find('%') else {
                break;
            };
            let name = &rest[start + 1..start + 1 + len];
            out.push_str(&rest[..start]);
            match lookup(name) {
                Some(v) if !name.is_empty() => out.push_str(&v),
                _ => out.push_str(&rest[start..start + len + 2]),
            }
            rest = &rest[start + len + 2..];
        }
        out.push_str(rest);
        out
    }

    /// The executable in an `ImagePath` command line (already expanded).
    /// Unquoted paths may contain spaces, so they run to the first `.exe`.
    pub fn image_program(image_path: &str) -> Option<String> {
        let s = image_path.trim();
        let program = if let Some(quoted) = s.strip_prefix('"') {
            quoted.split('"').next()?
        } else if let Some(end) = s.to_ascii_lowercase().find(".exe") {
            &s[..end + 4]
        } else {
            s.split_whitespace().next()?
        };
        let program = program.trim();
        (!program.is_empty()).then(|| program.to_string())
    }

    /// Builds a service from its key name and values. `windows_dir` is the
    /// Windows folder (`C:\Windows`); services running from there are part of
    /// Windows. Drivers and per-user service templates are skipped.
    pub fn parse_service(
        name: &str,
        v: &Values,
        windows_dir: &str,
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Option<Service> {
        let get = |k: &str| v.get(k).map(|s| s.trim()).filter(|s| !s.is_empty());
        let kind: u32 = get("Type")?.parse().ok()?;
        // 0x10 own process, 0x20 shared process; 0x40/0x80 mark per-user services.
        if kind & 0x30 == 0 || kind & 0xC0 != 0 {
            return None;
        }
        let image = expand_env(get("ImagePath")?, lookup);
        let lower = image.trim_start_matches('"').to_lowercase();
        let windows_dir = windows_dir.trim_end_matches('\\').to_lowercase();
        if lower.starts_with(&format!("{windows_dir}\\"))
            || lower.starts_with(r"\systemroot\")
            || lower.starts_with(r"system32\")
            || lower.starts_with(r"\??\")
        {
            return None;
        }
        let program = image_program(&image)?;
        let mut svc = Service::new(ServiceKind::Windows, name);
        svc.display_name = get("DisplayName")
            .filter(|d| !d.starts_with('@'))
            .map(String::from);
        svc.starts_automatically = get("Start") == Some("2");
        svc.program = Some(PathBuf::from(program));
        Some(svc)
    }

    #[cfg(windows)]
    pub fn scan(warnings: &mut Vec<String>) -> Vec<Service> {
        use winreg::RegKey;
        use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};

        let Ok(root) = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags(r"SYSTEM\CurrentControlSet\Services", KEY_READ)
        else {
            warnings.push("services: could not open the service list".into());
            return Vec::new();
        };
        let windows_dir = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let lookup = |n: &str| std::env::var(n).ok();
        root.enum_keys()
            .filter_map(Result::ok)
            .filter_map(|name| {
                let key = root.open_subkey_with_flags(&name, KEY_READ).ok()?;
                let values: Values = SERVICE_VALUES
                    .iter()
                    .filter_map(|&n| {
                        let v = key
                            .get_value::<String, _>(n)
                            .ok()
                            .or_else(|| key.get_value::<u32, _>(n).ok().map(|d| d.to_string()))?;
                        Some((n.to_string(), v))
                    })
                    .collect();
                parse_service(&name, &values, &windows_dir, lookup)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn parses_systemd_units() {
        let unit = "[Unit]\nDescription=OpenBSD Secure Shell server\n\n[Service]\nExecStartPre=/usr/sbin/sshd -t\nExecStart=-/usr/sbin/sshd -D $SSHD_OPTS\nExecStart=/bin/false\n";
        let (desc, program) = systemd::parse_unit(unit);
        assert_eq!(desc.as_deref(), Some("OpenBSD Secure Shell server"));
        assert_eq!(program.as_deref(), Some(Path::new("/usr/sbin/sshd")));
        assert_eq!(systemd::parse_unit("[Service]\nExecStart=sshd\n").1, None);
    }

    #[test]
    fn parses_cgroups() {
        assert_eq!(
            systemd::parse_cgroup("0::/system.slice/ssh.service\n").as_deref(),
            Some("ssh.service")
        );
        assert_eq!(
            systemd::parse_cgroup("12:pids:/\n1:name=systemd:/system.slice/cron.service\n")
                .as_deref(),
            Some("cron.service")
        );
        assert_eq!(systemd::parse_cgroup("0::/init.scope\n"), None);
    }

    #[cfg(unix)]
    #[test]
    fn scans_systemd_units() {
        use std::collections::HashSet;
        use std::fs;

        let tmp = tempfile::tempdir().unwrap();
        let etc = tmp.path().join("etc");
        let lib = tmp.path().join("lib");
        fs::create_dir_all(etc.join("multi-user.target.wants")).unwrap();
        fs::create_dir_all(&lib).unwrap();
        let unit = |desc: &str, exe: &str| {
            format!("[Unit]\nDescription={desc}\n[Service]\nExecStart={exe}\n")
        };
        fs::write(lib.join("ssh.service"), unit("SSH", "/usr/sbin/sshd -D")).unwrap();
        fs::write(lib.join("cups.service"), unit("CUPS", "/usr/sbin/cupsd -l")).unwrap();
        fs::write(lib.join("idle.service"), unit("Idle", "/usr/bin/idle")).unwrap();
        fs::write(lib.join("getty@.service"), unit("Getty", "/sbin/agetty")).unwrap();
        fs::write(lib.join("masked.service"), unit("Masked", "/usr/bin/m")).unwrap();
        std::os::unix::fs::symlink(
            lib.join("ssh.service"),
            etc.join("multi-user.target.wants/ssh.service"),
        )
        .unwrap();
        std::os::unix::fs::symlink(lib.join("ssh.service"), etc.join("sshd.service")).unwrap();
        std::os::unix::fs::symlink("/dev/null", etc.join("masked.service")).unwrap();

        let running: HashSet<String> = ["cups.service", "masked.service"].map(String::from).into();
        let found = systemd::scan_units(&[etc.as_path(), lib.as_path()], &etc, &running);
        let summary: Vec<(&str, bool, bool)> = found
            .iter()
            .map(|s| (s.name.as_str(), s.starts_automatically, s.running))
            .collect();
        assert_eq!(
            summary,
            [("cups.service", false, true), ("ssh.service", true, false)]
        );
        assert_eq!(found[1].display_name.as_deref(), Some("SSH"));
        assert_eq!(
            found[1].program.as_deref(),
            Some(Path::new("/usr/sbin/sshd"))
        );
    }

    #[test]
    fn parses_launchd_jobs() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>us.zoom.ZoomDaemon</string>
  <key>ProgramArguments</key><array><string>/Library/PrivilegedHelperTools/us.zoom.ZoomDaemon</string><string>--daemon</string></array>
  <key>KeepAlive</key><dict><key>SuccessfulExit</key><false/></dict>
</dict></plist>"#;
        let svc = launchd::parse_job(xml).unwrap();
        assert_eq!(svc.kind, ServiceKind::Launchd);
        assert_eq!(svc.name, "us.zoom.ZoomDaemon");
        assert!(svc.starts_automatically);
        assert_eq!(
            svc.program.as_deref(),
            Some(Path::new(
                "/Library/PrivilegedHelperTools/us.zoom.ZoomDaemon"
            ))
        );

        let disabled = br#"<plist version="1.0"><dict><key>Label</key><string>com.example.updater</string><key>Program</key><string>/Applications/Example.app/Contents/MacOS/updater</string><key>RunAtLoad</key><true/><key>Disabled</key><true/></dict></plist>"#;
        let svc = launchd::parse_job(disabled).unwrap();
        assert!(!svc.starts_automatically);
        assert!(launchd::parse_job(b"<plist version=\"1.0\"><dict/></plist>").is_none());
    }

    fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn parses_windows_services() {
        let env = |n: &str| match n.to_ascii_lowercase().as_str() {
            "systemroot" | "windir" => Some(r"C:\Windows".to_string()),
            "programfiles" => Some(r"C:\Program Files".to_string()),
            _ => None,
        };
        assert_eq!(
            windows::expand_env(r"%ProgramFiles%\App\%Unknown%\x.exe 50%", env),
            r"C:\Program Files\App\%Unknown%\x.exe 50%"
        );
        assert_eq!(
            windows::image_program(r#""C:\Program Files\Google\Update\GoogleUpdate.exe" /svc"#)
                .as_deref(),
            Some(r"C:\Program Files\Google\Update\GoogleUpdate.exe")
        );
        assert_eq!(
            windows::image_program(r"C:\Program Files\Vendor App\agent.EXE -k run").as_deref(),
            Some(r"C:\Program Files\Vendor App\agent.EXE")
        );

        let svc = windows::parse_service(
            "gupdate",
            &values(&[
                ("Type", "16"),
                ("Start", "2"),
                (
                    "ImagePath",
                    r#""%ProgramFiles%\Google\Update\GoogleUpdate.exe" /svc"#,
                ),
                ("DisplayName", "Google Update Service (gupdate)"),
            ]),
            r"C:\Windows",
            env,
        )
        .unwrap();
        assert_eq!(
            svc.display_name.as_deref(),
            Some("Google Update Service (gupdate)")
        );
        assert!(svc.starts_automatically);
        assert_eq!(
            svc.program.as_deref(),
            Some(Path::new(
                r"C:\Program Files\Google\Update\GoogleUpdate.exe"
            ))
        );

        let os = [
            ("Type", "32"),
            ("Start", "2"),
            ("ImagePath", r"%SystemRoot%\system32\svchost.exe -k netsvcs"),
        ];
        assert!(windows::parse_service("BITS", &values(&os), r"C:\Windows", env).is_none());
        let driver = [
            ("Type", "1"),
            ("Start", "0"),
            ("ImagePath", r"\SystemRoot\System32\drivers\x.sys"),
        ];
        assert!(windows::parse_service("x", &values(&driver), r"C:\Windows", env).is_none());
        let per_user = [
            ("Type", "96"),
            ("Start", "3"),
            ("ImagePath", r"C:\Program Files\A\a.exe"),
        ];
        assert!(windows::parse_service("a", &values(&per_user), r"C:\Windows", env).is_none());
        let manual = [
            ("Type", "16"),
            ("Start", "3"),
            ("ImagePath", r"C:\Tools\Svc\svc.exe"),
            ("DisplayName", "@%SystemRoot%\\x.dll,-1"),
        ];
        let svc = windows::parse_service("svc", &values(&manual), r"C:\Windows", env).unwrap();
        assert!(!svc.starts_automatically);
        assert_eq!(svc.display_name, None);
    }

    #[test]
    fn links_services_to_software_and_hides_programs() {
        let mut zoom = Software::new(Source::MacApp, "us.zoom.xos", "zoom.us");
        zoom.location = Some("/Applications/zoom.us.app".into());
        let openssh = Software::new(Source::Dpkg, "sshd", "sshd");
        let software = [zoom, openssh];

        let svc = |name: &str, program: &str| {
            let mut s = Service::new(ServiceKind::Launchd, name);
            s.program = Some(program.into());
            s
        };
        let mut services = vec![
            svc(
                "us.zoom.updater",
                "/Applications/zoom.us.app/Contents/Library/updater",
            ),
            svc("sshd", "/usr/sbin/sshd"),
            svc("orphan", "/Library/Helpers/orphan"),
            svc("orphan", "/Library/Helpers/orphan"),
        ];
        finish(&mut services, &software, &["/usr/sbin/sshd".into()]);
        let summary: Vec<(&str, Option<&str>, bool)> = services
            .iter()
            .map(|s| {
                (
                    s.name.as_str(),
                    s.software.as_ref().map(|r| r.id.as_str()),
                    s.running,
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("orphan", None, false),
                ("sshd", Some("sshd"), true),
                ("us.zoom.updater", Some("us.zoom.xos"), false),
            ]
        );
        let json = serde_json::to_string(&services).unwrap();
        assert!(
            !json.contains("/Applications") && !json.contains("/usr/sbin"),
            "{json}"
        );
    }
}
