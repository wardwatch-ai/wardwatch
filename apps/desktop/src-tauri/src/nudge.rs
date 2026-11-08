//! Runs the nudge engine (crates/wardwatch-nudge): read-only for Home and the
//! tray, and a background loop that sends the plan's system notifications.
//!
//! The loop wakes at the plan's `next_check` (at most an hour away, so a
//! laptop that slept catches up), or as soon as the Update plan is saved.
//! Quiet hours, the Sunday Tune-Up at 10:00 and holds are all the engine's;
//! this module only sends what it says and remembers that it did.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use jiff::{Timestamp, Zoned};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Notify;
use wardwatch_nudge::{ApprovedAlerts, Finding, History, Notification, NudgePlan};

use crate::plan::UpdatePlan;
use crate::{commands, fix, store};

const HISTORY_VERSION: u64 = 1;
const MIN_WAIT: Duration = Duration::from_secs(60);
const MAX_WAIT: Duration = Duration::from_secs(60 * 60);

/// The loop's wake-up call and the lock on the history file.
#[derive(Default)]
pub struct NudgeState {
    wake: Notify,
    history: Mutex<()>,
}

impl NudgeState {
    /// Re-plans now, for example after the Update plan changed.
    pub fn wake(&self) {
        self.wake.notify_one();
    }
}

/// The engine's view of the saved plan. Both sides use the same JSON shape,
/// so the stored form is converted through it rather than field by field.
fn engine_plan(stored: &UpdatePlan) -> Result<wardwatch_nudge::UpdatePlan, String> {
    let json = serde_json::to_vec(stored).map_err(|e| e.to_string())?;
    wardwatch_nudge::UpdatePlan::from_json(&json)
}

pub fn plan_for(
    findings: &[Finding],
    stored: &UpdatePlan,
    history: &History,
    now: &Zoned,
) -> Result<NudgePlan, String> {
    // Broad OS and browser alerts stay held until signed alerts.json exists.
    Ok(wardwatch_nudge::plan(
        findings,
        &engine_plan(stored)?,
        &ApprovedAlerts::none(),
        history,
        now,
    ))
}

/// One pass: plan, send each notification, record what went out. Returns
/// when to look again.
pub fn step(
    findings: &[Finding],
    stored: &UpdatePlan,
    history: &mut History,
    now: &Zoned,
    mut send: impl FnMut(&Notification),
) -> Result<Timestamp, String> {
    let plan = plan_for(findings, stored, history, now)?;
    for n in &plan.notify {
        send(n);
    }
    history.record(&plan, now.timestamp());
    Ok(plan.next_check)
}

#[derive(Serialize, Deserialize)]
struct StoredHistory {
    v: u64,
    #[serde(flatten)]
    history: History,
}

fn history_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("nudge-history.json"))
        .map_err(|e| format!("no config folder: {e}"))
}

/// A missing or unreadable file means nothing was sent yet.
fn load_history(path: &Path) -> History {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<StoredHistory>(&bytes).ok())
        .filter(|s| s.v == HISTORY_VERSION)
        .map(|s| s.history)
        .unwrap_or_default()
}

fn save_history(path: &Path, history: History) -> Result<(), String> {
    let stored = StoredHistory {
        v: HISTORY_VERSION,
        history,
    };
    store::save(path, &stored, HISTORY_VERSION, "nudge history")
}

/// The nudge plan as of now, for Home and the tray. Sends nothing.
pub fn current<R: Runtime>(app: &AppHandle<R>) -> Result<NudgePlan, String> {
    let stored = commands::load_plan(app)?;
    let findings = fix::list_findings(app)?;
    let history = {
        let state = app.state::<NudgeState>();
        let _guard = state.history.lock().map_err(|e| e.to_string())?;
        load_history(&history_path(app)?)
    };
    plan_for(&findings, &stored, &history, &Zoned::now())
}

fn run_once<R: Runtime>(app: &AppHandle<R>) -> Result<Timestamp, String> {
    let stored = commands::load_plan(app)?;
    let findings = fix::list_findings(app)?;
    let state = app.state::<NudgeState>();
    let _guard = state.history.lock().map_err(|e| e.to_string())?;
    let path = history_path(app)?;
    let mut history = load_history(&path);
    let next = step(&findings, &stored, &mut history, &Zoned::now(), |n| {
        if let Err(e) = app
            .notification()
            .builder()
            .title(&n.title)
            .body(&n.body)
            .show()
        {
            eprintln!("wardwatch: could not show a notification: {e}");
        }
    })?;
    save_history(&path, history)?;
    Ok(next)
}

fn wait_until(next: Timestamp) -> Duration {
    let ms = next.duration_since(Timestamp::now()).as_millis();
    Duration::from_millis(u64::try_from(ms).unwrap_or(0)).clamp(MIN_WAIT, MAX_WAIT)
}

/// Starts the background loop. Without a catalog yet (every release build for
/// now) each pass fails quietly and the loop tries again within the hour.
pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        loop {
            let handle = app.clone();
            let wait = match tauri::async_runtime::spawn_blocking(move || run_once(&handle)).await {
                Ok(Ok(next)) => wait_until(next),
                Ok(Err(e)) => {
                    eprintln!("wardwatch: nudges not checked: {e}");
                    MAX_WAIT
                }
                Err(e) => {
                    eprintln!("wardwatch: nudge check stopped: {e}");
                    MAX_WAIT
                }
            };
            let state = app.state::<NudgeState>();
            tokio::select! {
                () = tokio::time::sleep(wait) => {}
                () = state.wake.notified() => {}
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exploited_app() -> Finding {
        serde_json::from_value(serde_json::json!({
            "key": "zoom", "app_id": "zoom", "app_name": "Zoom", "kind": "app",
            "source": "mac_app", "software_id": "us.zoom.xos", "name": "Zoom",
            "browser": null, "version_scheme": "dotted", "installed": "1.0", "running": false,
            "installs": [{"source": "mac_app", "id": "us.zoom.xos", "name": "Zoom",
                          "version": "1.0", "running": false}],
            "fixed_version": "2.0", "latest": "2.0", "update_available": true,
            "exploited": true, "severity": "high",
            "vulns": [{"id": "CVE-1", "aliases": [], "published": "2026-11-01",
                       "severity": "high", "cvss": null, "exploited": true,
                       "kev": null, "summary": "", "fixed": "2.0"}],
        }))
        .unwrap()
    }

    fn at(s: &str) -> Zoned {
        format!("{s}[America/New_York]").parse().unwrap()
    }

    #[test]
    fn the_saved_plan_reads_as_the_engines_plan() {
        let mut stored = UpdatePlan::default();
        stored.holds.push(crate::plan::Hold {
            source: "mac_app".into(),
            id: "com.apple.FinalCut".into(),
            since: "2026-11-02".into(),
        });
        let engine = engine_plan(&stored).unwrap();
        assert_eq!(
            wardwatch_nudge::Preset::matching(&engine.rules),
            Some(wardwatch_nudge::Preset::Balanced)
        );
        assert!(engine.is_held("mac_app", "com.apple.FinalCut"));
    }

    #[test]
    fn sends_once_then_remembers_it() {
        let findings = [exploited_app()];
        let plan = UpdatePlan::default();
        let mut history = History::default();
        let mut sent = Vec::new();
        // Tuesday 14:00, outside quiet hours.
        step(
            &findings,
            &plan,
            &mut history,
            &at("2026-11-10T14:00"),
            |n| {
                sent.push(n.title.clone());
            },
        )
        .unwrap();
        assert_eq!(sent.len(), 1);
        step(
            &findings,
            &plan,
            &mut history,
            &at("2026-11-10T14:05"),
            |n| {
                sent.push(n.title.clone());
            },
        )
        .unwrap();
        assert_eq!(sent.len(), 1, "no repeat within the reminder interval");
    }

    #[test]
    fn quiet_hours_and_holds_send_nothing() {
        let findings = [exploited_app()];
        let mut history = History::default();
        let mut sent = 0;
        step(
            &findings,
            &UpdatePlan::default(),
            &mut history,
            &at("2026-11-10T23:30"),
            |_| sent += 1,
        )
        .unwrap();
        let mut held = UpdatePlan::default();
        held.holds.push(crate::plan::Hold {
            source: "mac_app".into(),
            id: "us.zoom.xos".into(),
            since: "2026-11-02".into(),
        });
        step(
            &findings,
            &held,
            &mut history,
            &at("2026-11-10T14:00"),
            |_| {
                sent += 1;
            },
        )
        .unwrap();
        assert_eq!(sent, 0);
    }

    #[test]
    fn history_round_trips_and_bad_files_start_fresh() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nudge-history.json");
        assert_eq!(load_history(&path), History::default());
        let mut history = History::default();
        history
            .notified
            .insert("zoom".into(), "2026-11-10T19:00:00Z".parse().unwrap());
        save_history(&path, history.clone()).unwrap();
        assert_eq!(load_history(&path), history);
        std::fs::write(&path, b"not json").unwrap();
        assert_eq!(load_history(&path), History::default());
    }

    #[test]
    fn waits_between_a_minute_and_an_hour() {
        assert_eq!(wait_until(Timestamp::now()), MIN_WAIT);
        let far = Timestamp::now() + jiff::SignedDuration::from_hours(30);
        assert_eq!(wait_until(far), MAX_WAIT);
    }
}
