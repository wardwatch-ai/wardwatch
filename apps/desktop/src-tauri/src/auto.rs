//! Auto: installs the updates the Update plan says to install by itself
//! (the nudge plan's `install` list), under the fix screen's rules, and keeps
//! a log the person can read in Settings.
//!
//! The rules, on top of the engine's (outside quiet hours, never for a held
//! program):
//! - Only one-click package-manager updates that need no password. Anything
//!   that would show an OS permission prompt, open an app or the OS's update
//!   screen, or touch a program that's open, waits for the person instead,
//!   with a notification that says why.
//! - It runs through `fix::run_fix`, which rescans, replans, and refuses if
//!   the plan differs from the one decided here; the updater never removes
//!   packages; and only a rescan that finds the hole closed counts as fixed.
//! - A program that failed or was skipped isn't tried again for a day.

use std::path::{Path, PathBuf};

use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use wardwatch_feed::Finding;
use wardwatch_update::{Admin, Check, Method, NoPlan, Outcome, Plan};

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
    let log: Log = serde_json::from_slice(&bytes)
        .map_err(|e| format!("the automatic install log is unreadable: {e}"))?;
    if log.v > LOG_VERSION {
        return Err("the automatic install log was saved by a newer version of Wardwatch".into());
    }
    Ok(log.entries)
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
/// prompt, nothing to open, and the program isn't in use. `Err` says what the
/// person needs to do instead.
pub fn allowed(finding: &Finding, plan: &Plan) -> Result<(), String> {
    if !matches!(plan.method, Method::Command { .. }) || !plan.one_click {
        return Err(format!("{} updates it, and you finish it there", plan.by));
    }
    if plan.admin != Admin::No {
        return Err("it needs your permission to install".into());
    }
    if finding.running || finding.installs.iter().any(|i| i.running) {
        return Err("it's open right now".into());
    }
    Ok(())
}

/// Whether `key` was tried (and didn't install) within [`RETRY_AFTER`].
fn waiting(entries: &[Entry], key: &str, now: Timestamp) -> bool {
    entries
        .iter()
        .rev()
        .find(|e| e.key == key)
        .is_some_and(|e| e.result != Status::Installed && now.duration_since(e.at) < RETRY_AFTER)
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
        (Outcome::Installed, _) => (
            Status::Failed,
            "it installed, but the hole still shows as open".into(),
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

/// Installs each of `keys` that's allowed, logs every attempt or skip, and
/// tells the person through `notify` (title, body, the program's key).
pub fn run<R: Runtime>(
    app: &AppHandle<R>,
    keys: &[String],
    findings: &[Finding],
    mut notify: impl FnMut(&str, &str, &str),
) -> Result<(), String> {
    if keys.is_empty() {
        return Ok(());
    }
    let path = log_path(app)?;
    let mut entries = load(&path)?;
    // Make sure attempts can be logged before making any.
    save(&path, entries.clone())?;
    for key in keys {
        let now = Timestamp::now();
        let Some(finding) = findings.iter().find(|f| &f.key == key) else {
            continue;
        };
        if waiting(&entries, key, now) || fix::is_busy(app) {
            continue;
        }
        let view = fix::get_fix(app, &finding.source, &finding.software_id)?;
        let mut entry = Entry {
            at: now,
            key: key.clone(),
            name: view.finding.app_name.clone(),
            from: view.finding.installed.clone(),
            to: view
                .finding
                .fixed_version
                .clone()
                .or(view.finding.latest.clone()),
            result: Status::Skipped,
            why: String::new(),
        };
        let decision = match (&view.plan, &view.no_plan) {
            (Some(plan), _) => allowed(&view.finding, plan).map(|()| plan.method.clone()),
            (None, no_plan) => Err(no_plan.as_ref().map_or_else(
                || "there's no way to update it here".into(),
                NoPlan::to_string,
            )),
        };
        match decision {
            Err(why) => entry.why = why,
            // run_fix replans from a fresh scan and refuses if `method` differs.
            Ok(method) => {
                match fix::run_fix(app, &finding.source, &finding.software_id, &method, |_| {}) {
                    Ok(result) => (entry.result, entry.why) = describe(&result),
                    Err(e) => (entry.result, entry.why) = (Status::Failed, e),
                }
            }
        }
        eprintln!(
            "wardwatch: automatic install of {key}: {:?} {}",
            entry.result, entry.why
        );
        let (title, body) = message(&entry);
        notify(&title, &body, key);
        entries.push(entry);
        save(&path, entries.clone())?;
    }
    Ok(())
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
        assert!(waiting(&log, "zoom", now));
        assert!(!waiting(&log, "vlc", now));
        let old = [entry("zoom", Status::Failed, "2026-11-09T08:00:00Z")];
        assert!(!waiting(&old, "zoom", now));
        let done = [entry("zoom", Status::Installed, "2026-11-10T08:00:00Z")];
        assert!(!waiting(&done, "zoom", now));
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
