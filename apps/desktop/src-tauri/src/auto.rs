//! Auto: installs the updates the Update plan says to install by itself
//! (the nudge plan's `install` list), under the fix screen's rules, and keeps
//! a log the person can read in Settings.
//!
//! The rules, on top of the engine's (outside quiet hours, never for a held
//! program):
//! - Only one-click package-manager updates that need no password, while
//!   the device is plugged in. Anything that would show an OS permission
//!   prompt, open an app or the OS's update screen, or touch a program that's
//!   open, waits for the person instead, with a notification that says why.
//! - It runs through `fix::run_checked`, the fix screen's runner: those rules
//!   are judged on the plan and program from the scan that runs it, the
//!   updater never removes packages, and only a rescan that finds the hole
//!   closed counts as fixed.
//! - A program that failed or was skipped isn't tried again for a day; one
//!   that waited on another update is tried again in a few minutes.

use std::path::{Path, PathBuf};

use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use wardwatch_feed::Finding;
use wardwatch_update::{Admin, Check, Method, Outcome, Plan, Tool};

use crate::{fix, store};

const LOG_VERSION: u64 = 1;
/// Entries kept; older ones are dropped.
const LOG_LIMIT: usize = 200;
/// How long a failed or skipped program waits before the next try.
const RETRY_AFTER: SignedDuration = SignedDuration::from_hours(24);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Installed, and the rescan found the hole closed.
    Installed,
    /// Waiting for the person; `why` says what they need to do.
    Skipped,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub at: Timestamp,
    /// The catalog app id.
    pub key: String,
    pub name: String,
    pub from: String,
    pub to: Option<String>,
    pub result: Status,
    pub why: String,
}

#[derive(Default, Serialize, Deserialize)]
struct Log {
    v: u64,
    entries: Vec<Entry>,
}

fn log_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("auto-installs.json"))
        .map_err(|e| format!("no config folder: {e}"))
}

fn load(path: &Path) -> Result<Vec<Entry>, String> {
    let Ok(bytes) = std::fs::read(path) else {
        return Ok(Vec::new());
    };
    match serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|v| v.get("v").and_then(serde_json::Value::as_u64))
    {
        Some(v) if v > LOG_VERSION => {
            Err("the automatic install log was saved by a newer version of Wardwatch".into())
        }
        // A damaged log starts again rather than stopping Auto for good.
        _ => Ok(serde_json::from_slice::<Log>(&bytes)
            .map(|log| log.entries)
            .unwrap_or_else(|_| {
                eprintln!("wardwatch: starting a fresh automatic install log");
                Vec::new()
            })),
    }
}

fn save(path: &Path, mut entries: Vec<Entry>) -> Result<(), String> {
    let excess = entries.len().saturating_sub(LOG_LIMIT);
    entries.drain(..excess);
    let log = Log {
        v: LOG_VERSION,
        entries,
    };
    store::save(path, &log, LOG_VERSION, "automatic install log")
}

/// Newest first, for Settings.
pub fn entries<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<Entry>, String> {
    let mut entries = load(&log_path(app)?)?;
    entries.reverse();
    Ok(entries)
}

/// Whether this plan may run without the person: one click, no password
/// prompt, nothing to open, and the program isn't in use. `Err` says what the person needs to do instead. Checked on the
/// plan and program exactly as the scan that runs it found them.
pub fn allowed(finding: &Finding, plan: &Plan) -> Result<(), String> {
    if !matches!(plan.method, Method::Command { .. }) || !plan.one_click {
        return Err(format!("{} updates it, and you finish it there", plan.by));
    }
    if plan.admin != Admin::No {
        return Err("it needs your permission to install".into());
    }
    // It downloads from wherever the AppImage says and replaces a file the
    // person put there themselves, so a person starts it.
    if matches!(
        plan.method,
        Method::Command {
            tool: Tool::AppImageUpdate,
            ..
        }
    ) {
        return Err("it replaces a file you downloaded yourself".into());
    }
    if finding.running || finding.installs.iter().any(|i| i.running) {
        return Err("it's open right now".into());
    }
    Ok(())
}

/// [`allowed`], and the device is plugged in.
fn allowed_now(finding: &Finding, plan: &Plan) -> Result<(), String> {
    allowed(finding, plan)?;
    if on_battery() {
        return Err("it waits until your device is plugged in".into());
    }
    Ok(())
}

/// Whether the device is running on battery. Where we can't tell, it counts
/// as on battery, so Auto waits for the person.
fn on_battery() -> bool {
    if cfg!(target_os = "macos") {
        // First line: "Now drawing from 'AC Power'" or "'Battery Power'".
        return std::process::Command::new("/usr/bin/pmset")
            .args(["-g", "batt"])
            .output()
            .map_or(true, |o| {
                !String::from_utf8_lossy(&o.stdout).contains("'AC Power'")
            });
    }
    if cfg!(target_os = "linux") {
        return linux_on_battery(Path::new("/sys/class/power_supply"));
    }
    true
}

/// On battery when there's a battery and no mains supply online. A desktop
/// with no battery is plugged in.
fn linux_on_battery(dir: &Path) -> bool {
    let read = |p: PathBuf| std::fs::read_to_string(p).unwrap_or_default();
    let mut battery = false;
    for supply in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = supply.path();
        match read(path.join("type")).trim() {
            "Mains" if read(path.join("online")).trim() == "1" => return false,
            "Battery" => battery = true,
            _ => {}
        }
    }
    battery
}

/// When `key` may be tried again, if its last try (that didn't install) was
/// within [`RETRY_AFTER`]. A time in the future (the clock moved back) doesn't count.
fn retry_at(entries: &[Entry], key: &str, now: Timestamp) -> Option<Timestamp> {
    let last = entries.iter().rev().find(|e| e.key == key)?;
    let at = last.at + RETRY_AFTER;
    (last.result != Status::Installed && last.at <= now && now < at).then_some(at)
}

/// What the person is told about one automatic install.
pub fn message(entry: &Entry) -> (String, String) {
    match entry.result {
        Status::Installed => (
            format!("Updated {}", entry.name),
            format!(
                "Wardwatch installed {} {} by itself, as your Update plan says, and checked the hole is closed.",
                entry.name,
                entry.to.as_deref().unwrap_or("")
            ),
        ),
        Status::Skipped => (
            format!("{} needs you", entry.name),
            format!(
                "Wardwatch didn't update {} by itself: {}. Update it from Wardwatch when you're ready.",
                entry.name, entry.why
            ),
        ),
        Status::Failed => (
            format!("{} didn't update", entry.name),
            format!(
                "Wardwatch tried to update {} by itself and it didn't work: {}. Have a look in Wardwatch.",
                entry.name, entry.why
            ),
        ),
    }
}

fn describe(result: &fix::FixResult) -> (Status, String) {
    match (&result.outcome, result.check) {
        (Outcome::Installed, Some(Check::Closed | Check::OnlyUnfixedLeft)) => {
            (Status::Installed, String::new())
        }
        (Outcome::Installed, Some(Check::StillOpen)) => (
            Status::Failed,
            "it installed, but the hole still shows as open".into(),
        ),
        (Outcome::Installed, _) => (
            Status::Failed,
            "it installed, but Wardwatch couldn't find it again to check".into(),
        ),
        (Outcome::NeedsMoreChanges { .. }, _) => (
            Status::Skipped,
            "the update would remove other software, which Wardwatch never does".into(),
        ),
        (Outcome::NothingNewer { .. }, _) => {
            (Status::Failed, "the updater has nothing newer yet".into())
        }
        (Outcome::Cancelled, _) => (Status::Skipped, "the install was cancelled".into()),
        (Outcome::Opened, _) => (Status::Skipped, "it needs you to finish it".into()),
        (Outcome::Failed { message, .. }, _) => (Status::Failed, message.clone()),
    }
}

/// How soon to look again after another update was running.
const BUSY_RETRY: SignedDuration = SignedDuration::from_mins(5);

/// Installs each of `keys` that's allowed, logs every attempt or skip, and
/// tells the person through `notify` (title, body, the program's key).
/// Returns when a program that didn't install should be tried again.
pub fn run<R: Runtime>(
    app: &AppHandle<R>,
    keys: &[String],
    findings: &[Finding],
    mut notify: impl FnMut(&str, &str, &str),
) -> Result<Option<Timestamp>, String> {
    if keys.is_empty() {
        return Ok(None);
    }
    let path = log_path(app)?;
    let mut entries = load(&path)?;
    // Make sure attempts can be logged before making any.
    save(&path, entries.clone())?;
    let mut next: Option<Timestamp> = None;
    let mut retry = |at: Timestamp| next = Some(next.map_or(at, |n| n.min(at)));
    for key in keys {
        let now = Timestamp::now();
        let Some(finding) = findings.iter().find(|f| &f.key == key) else {
            continue;
        };
        if let Some(at) = retry_at(&entries, key, now) {
            retry(at);
            continue;
        }
        // `allowed_now` judges the plan and program from run_checked's own scan,
        // so what it approves is exactly what runs.
        let ran = fix::run_checked(
            app,
            &finding.source,
            &finding.software_id,
            allowed_now,
            |_| {},
        );
        let (result, why) = match ran {
            Ok(result) => describe(&result),
            Err(fix::Refused::Busy) => {
                // The fix screen is updating something; not a failure.
                retry(now + BUSY_RETRY);
                continue;
            }
            Err(fix::Refused::NotAccepted(why)) => (Status::Skipped, why),
            Err(fix::Refused::Error(why)) => (Status::Failed, why),
        };
        if result != Status::Installed {
            retry(now + RETRY_AFTER);
        }
        let entry = Entry {
            at: now,
            key: key.clone(),
            name: finding.app_name.clone(),
            from: finding.installed.clone(),
            to: finding.fixed_version.clone().or(finding.latest.clone()),
            result,
            why,
        };
        eprintln!(
            "wardwatch: automatic install of {key}: {:?} {}",
            entry.result, entry.why
        );
        let (title, body) = message(&entry);
        notify(&title, &body, key);
        entries.push(entry);
        save(&path, entries.clone())?;
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, result: Status, at: &str) -> Entry {
        Entry {
            at: at.parse().unwrap(),
            key: key.into(),
            name: "Zoom".into(),
            from: "6.4.2".into(),
            to: Some("6.5.0".into()),
            result,
            why: "it's open right now".into(),
        }
    }

    fn zoom(running: bool) -> Finding {
        serde_json::from_value(serde_json::json!({
            "key": "zoom", "app_id": "zoom", "app_name": "Zoom", "kind": "app",
            "source": "dpkg", "software_id": "zoom", "name": "Zoom",
            "browser": null, "version_scheme": "dotted", "installed": "1.0", "running": running,
            "installs": [{"source": "dpkg", "id": "zoom", "name": "Zoom",
                          "version": "1.0", "running": running}],
            "fixed_version": "2.0", "latest": "2.0", "update_available": true,
            "exploited": true, "severity": "high", "vulns": [],
        }))
        .unwrap()
    }

    fn plan(method: Method, admin: Admin) -> Plan {
        Plan {
            method,
            by: "apt".into(),
            one_click: true,
            admin,
            steps: vec![],
            verifies: String::new(),
            manual: None,
            restart_note: None,
        }
    }

    #[test]
    fn only_quiet_one_click_updates_of_closed_programs_run() {
        let apt = || Method::Command {
            tool: wardwatch_update::Tool::Apt,
            argv: vec!["apt-get".into()],
        };
        assert_eq!(allowed(&zoom(false), &plan(apt(), Admin::No)), Ok(()));
        // Anything that could show a password prompt waits for the person.
        assert!(allowed(&zoom(false), &plan(apt(), Admin::Asks)).is_err());
        assert!(allowed(&zoom(false), &plan(apt(), Admin::MayAsk)).is_err());
        assert!(allowed(&zoom(true), &plan(apt(), Admin::No)).is_err());
        let appimage = Method::Command {
            tool: Tool::AppImageUpdate,
            argv: vec!["appimageupdatetool".into()],
        };
        assert!(allowed(&zoom(false), &plan(appimage, Admin::No)).is_err());
        let opens = Method::AppUpdater { open_bundle: None };
        assert!(allowed(&zoom(false), &plan(opens, Admin::No)).is_err());
    }

    #[test]
    fn only_a_closed_hole_counts_as_installed() {
        let result = |outcome, check| fix::FixResult {
            outcome,
            check,
            finding: None,
        };
        for (check, status) in [
            (Some(Check::Closed), Status::Installed),
            (Some(Check::OnlyUnfixedLeft), Status::Installed),
            (Some(Check::StillOpen), Status::Failed),
            (Some(Check::NotFound), Status::Failed),
            (None, Status::Failed),
        ] {
            assert_eq!(describe(&result(Outcome::Installed, check)).0, status);
        }
        let removes = result(Outcome::NeedsMoreChanges { tail: vec![] }, None);
        assert_eq!(describe(&removes).0, Status::Skipped);
    }

    #[test]
    fn failed_or_skipped_programs_wait_a_day() {
        let now: Timestamp = "2026-11-10T12:00:00Z".parse().unwrap();
        let log = [entry("zoom", Status::Skipped, "2026-11-10T08:00:00Z")];
        assert_eq!(
            retry_at(&log, "zoom", now),
            Some("2026-11-11T08:00:00Z".parse().unwrap())
        );
        assert_eq!(retry_at(&log, "vlc", now), None);
        let old = [entry("zoom", Status::Failed, "2026-11-09T08:00:00Z")];
        assert_eq!(retry_at(&old, "zoom", now), None);
        let done = [entry("zoom", Status::Installed, "2026-11-10T08:00:00Z")];
        assert_eq!(retry_at(&done, "zoom", now), None);
        // Logged while the clock was ahead: don't wait on a future time.
        let ahead = [entry("zoom", Status::Failed, "2026-12-01T08:00:00Z")];
        assert_eq!(retry_at(&ahead, "zoom", now), None);
    }

    #[test]
    fn linux_counts_battery_only_without_mains() {
        let dir = tempfile::tempdir().unwrap();
        let supply = |name: &str, kind: &str, online: Option<&str>| {
            let p = dir.path().join(name);
            std::fs::create_dir(&p).unwrap();
            std::fs::write(p.join("type"), kind).unwrap();
            if let Some(o) = online {
                std::fs::write(p.join("online"), o).unwrap();
            }
        };
        assert!(!linux_on_battery(dir.path()), "a desktop has no battery");
        supply("BAT0", "Battery\n", None);
        supply("AC", "Mains\n", Some("0\n"));
        assert!(linux_on_battery(dir.path()));
        std::fs::write(dir.path().join("AC/online"), "1\n").unwrap();
        assert!(!linux_on_battery(dir.path()));
    }

    #[test]
    fn the_log_round_trips_keeps_the_newest_and_refuses_newer_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auto-installs.json");
        assert!(load(&path).unwrap().is_empty());
        let many: Vec<_> = (0..LOG_LIMIT + 5)
            .map(|i| {
                let mut e = entry("zoom", Status::Installed, "2026-11-10T08:00:00Z");
                e.from = i.to_string();
                e
            })
            .collect();
        save(&path, many).unwrap();
        let back = load(&path).unwrap();
        assert_eq!(back.len(), LOG_LIMIT);
        assert_eq!(back[0].from, "5");
        std::fs::write(&path, b"{not json").unwrap();
        assert!(
            load(&path).unwrap().is_empty(),
            "a damaged log starts again"
        );
        std::fs::write(&path, br#"{"v":2,"entries":[]}"#).unwrap();
        assert!(load(&path).is_err());
    }

    #[test]
    fn messages_say_what_happened_and_what_to_do() {
        let (title, body) = message(&entry("zoom", Status::Skipped, "2026-11-10T08:00:00Z"));
        assert_eq!(title, "Zoom needs you");
        assert!(body.contains("it's open right now"));
        let (title, _) = message(&entry("zoom", Status::Installed, "2026-11-10T08:00:00Z"));
        assert_eq!(title, "Updated Zoom");
    }
}
